use anyhow::{Context, Result};
use git2::{
    Cred, FetchOptions, RemoteCallbacks, Repository, ResetType, SubmoduleUpdateOptions,
    build::RepoBuilder,
};
use std::path::Path;

/// Build a `FetchOptions` with SSH-agent credentials enabled.
///
/// This allows Git operations to authenticate using the user's SSH agent.
/// If no SSH key is found, it falls back to default credentials.
fn fetch_opts_with_creds() -> FetchOptions<'static> {
    let mut cb = RemoteCallbacks::new();
    cb.credentials(|_url, username_from_url, _allowed| {
        Cred::ssh_key_from_agent(username_from_url.unwrap_or("git")).or_else(|_| Cred::default())
    });

    let mut fo = FetchOptions::new();
    fo.remote_callbacks(cb);
    fo
}

/// Initialize and update all submodules for the given repository.
///
/// This ensures that nested submodules (e.g. plugins that depend on other repos)
/// are cloned and checked out at the correct revision.
///
/// # Errors
/// Returns an error if any submodule fails to initialize or update.
fn update_submodules(repo: &Repository) -> Result<()> {
    let mut subs = repo.submodules().unwrap_or_default();
    for sm in subs.iter_mut() {
        sm.init(true)?;
        let mut opt = SubmoduleUpdateOptions::new();
        sm.update(true, Some(&mut opt))?;
    }
    Ok(())
}

/// Ensure that a repository exists at the given path.
///
/// - If the repository already exists:
///   - Performs `git fetch origin`
///   - Resets to the specified revision (if provided)
///   - Otherwise resets to the current HEAD
///   - Updates submodules
///
/// - If the repository does not exist:
///   - Clones it from the given URL
///   - Optionally checks out the specified revision
///   - Updates submodules
///
/// # Arguments
/// - `url`: Remote Git URL (e.g., GitHub repo)
/// - `dest`: Local directory path where the repo should exist
/// - `rev`: Optional branch, tag, or commit SHA to checkout
///
/// # Errors
/// - Returns an error if cloning, fetching, or checkout fails.
pub fn ensure_repo(url: &str, dest: &Path, rev: Option<&str>) -> Result<()> {
    if dest.join(".git").exists() {
        let repo = Repository::open(dest)?;
        fetch_origin(&repo)?;
        if let Some(r) = rev {
            checkout_rev(&repo, r)?;
        } else {
            let head = repo.head()?.peel_to_commit()?;
            repo.reset(head.as_object(), ResetType::Hard, None)?;
        }
        update_submodules(&repo)?;
        Ok(())
    } else {
        let mut builder = RepoBuilder::new();
        builder.fetch_options(fetch_opts_with_creds());

        let repo = builder
            .clone(url, dest)
            .with_context(|| format!("git clone {}", url))?;

        if let Some(r) = rev {
            checkout_rev(&repo, r)?;
        }
        update_submodules(&repo)?;
        Ok(())
    }
}

/// Perform `git fetch origin` to update remote refs.
///
/// This fetches both branches and tags from `origin` into the local repository.
///
/// # Errors
/// Returns an error if the fetch operation fails.
pub fn fetch_origin(repo: &Repository) -> Result<()> {
    let mut fo = fetch_opts_with_creds();

    let mut remote = repo.find_remote("origin")?;
    remote
        .fetch(
            &[
                "refs/heads/*:refs/remotes/origin/*",
                "refs/tags/*:refs/tags/*",
            ],
            Some(&mut fo),
            None,
        )
        .context("git fetch origin")?;
    Ok(())
}

/// Checkout a specific revision (tag, branch, or commit).
///
/// - Tries to resolve as a tag (`refs/tags/<rev>`)
/// - Then as a remote branch (`refs/remotes/origin/<rev>`)
/// - Finally as a raw commit SHA
///
/// The repository will be placed into a detached HEAD state.
///
/// # Errors
/// Returns an error if the revision cannot be resolved.
pub fn checkout_rev(repo: &Repository, rev: &str) -> Result<()> {
    let obj = repo
        .revparse_single(&format!("refs/tags/{}", rev))
        .or_else(|_| repo.revparse_single(&format!("refs/remotes/origin/{}", rev)))
        .or_else(|_| repo.revparse_single(rev))
        .with_context(|| format!("rev not found: {}", rev))?;

    repo.checkout_tree(&obj, None)?;
    repo.set_head_detached(obj.id())?;
    Ok(())
}
