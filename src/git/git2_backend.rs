use anyhow::{Context, Result, anyhow};
use git2::{
    BranchType, Cred, FetchOptions, ObjectType, Reference, RemoteCallbacks, Repository, ResetType,
    SubmoduleUpdateOptions,
    build::{CheckoutBuilder, RepoBuilder},
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

/// Attach to the remote's default branch (origin/HEAD), creating a local
/// tracking branch if necessary, and hard-reset to the remote tip.
///
/// Fallbacks are tried in order if `origin/HEAD` is missing:
/// `refs/remotes/origin/main` → `refs/remotes/origin/master`.
///
/// # Errors
/// Returns an error if no suitable default branch can be found or checkout fails.
fn attach_default_branch(repo: &Repository) -> Result<()> {
    let target_remote_ref = if let Ok(origin_head) = repo.find_reference("refs/remotes/origin/HEAD")
    {
        origin_head
            .symbolic_target()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("origin/HEAD has no symbolic target"))?
    } else if repo.find_reference("refs/remotes/origin/main").is_ok() {
        "refs/remotes/origin/main".to_string()
    } else if repo.find_reference("refs/remotes/origin/master").is_ok() {
        "refs/remotes/origin/master".to_string()
    } else {
        return Err(anyhow!(
            "could not determine default branch (missing origin/HEAD, origin/main, origin/master)"
        ));
    };

    let branch_name = target_remote_ref
        .strip_prefix("refs/remotes/origin/")
        .ok_or_else(|| anyhow!("unexpected remote ref: {}", target_remote_ref))?;

    let remote_tip = repo.find_reference(&target_remote_ref)?.peel_to_commit()?;

    let local_ref = match repo.find_branch(branch_name, BranchType::Local) {
        Ok(b) => b.into_reference(),
        Err(_) => {
            let mut b = repo.branch(branch_name, &remote_tip, true)?;
            b.set_upstream(Some(&format!("origin/{}", branch_name)))?;
            b.into_reference()
        }
    };

    repo.set_head(
        local_ref
            .name()
            .ok_or_else(|| anyhow!("invalid reference name"))?,
    )?;
    repo.reset(remote_tip.as_object(), ResetType::Hard, None)?;
    repo.checkout_head(Some(CheckoutBuilder::new().force()))?;
    Ok(())
}

/// Ensure that a repository exists at the given path.
///
/// - If the repository already exists:
///   - Performs `git fetch origin`
///   - If `rev` is Some: checkout that revision (branch→attach / tag・SHA→detached)
///   - If `rev` is None: **attach to the remote's default branch** (origin/HEAD)
///   - Updates submodules
///
/// - If the repository does not exist:
///   - Clones it from the given URL
///   - If `rev` is Some: checkout that revision
///   - If `rev` is None: **attach to the remote's default branch**
///   - Updates submodules
///
/// # Errors
/// Returns an error if cloning, fetching, or checkout fails.
pub fn ensure_repo(url: &str, dest: &Path, rev: Option<&str>) -> Result<()> {
    if dest.join(".git").exists() {
        let repo = Repository::open(dest)?;
        fetch_origin(&repo)?;
        if let Some(r) = rev {
            checkout_rev(&repo, r)?;
        } else {
            attach_default_branch(&repo)?;
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
        } else {
            fetch_origin(&repo)?;
            attach_default_branch(&repo)?;
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

/// Attach HEAD to the given branch reference and update the working tree.
///
/// Moves HEAD to the provided branch ref (attached state) and checks out
/// the branch tip into the worktree.
///
/// # Errors
/// Returns an error if the reference has no valid name, or if updating
/// HEAD or checking out the branch fails.
fn checkout_attach_to_reference(repo: &Repository, reference: &Reference) -> Result<()> {
    let name = reference
        .name()
        .ok_or_else(|| anyhow!("invalid reference name"))?;
    repo.set_head(name)?;
    repo.checkout_head(Some(CheckoutBuilder::new().force()))?;
    Ok(())
}

/// Checkout a specific revision (branch, tag, or commit).
///
/// Resolution order:
/// 1. Local branch (`refs/heads/<rev>`) → attach HEAD to the branch
/// 2. Remote branch (`refs/remotes/origin/<rev>`) → create/attach a local tracking branch
/// 3. Tag (`refs/tags/<rev>`) → peel to the commit and detach HEAD
/// 4. Commit SHA or revspec → peel to the commit and detach HEAD
///
/// - Branches are checked out in an **attached** state (HEAD tracks the branch).
/// - Tags and raw commits are checked out in a **detached** state.
///
/// # Errors
/// Returns an error if the revision cannot be resolved or if checkout fails.
pub fn checkout_rev(repo: &Repository, rev: &str) -> Result<()> {
    if let Ok(reference) = repo.find_reference(&format!("refs/heads/{}", rev)) {
        checkout_attach_to_reference(repo, &reference)?;
        return Ok(());
    }

    if let Ok(remote_ref) = repo.find_reference(&format!("refs/remotes/origin/{}", rev)) {
        let target_commit = remote_ref.peel_to_commit()?;

        let reference = match repo.find_branch(rev, BranchType::Local) {
            Ok(b) => b.into_reference(),
            Err(_) => {
                let mut b = repo.branch(rev, &target_commit, true)?;
                b.set_upstream(Some(&format!("origin/{}", rev)))?;
                b.into_reference()
            }
        };

        repo.reset(target_commit.as_object(), ResetType::Hard, None)?;
        checkout_attach_to_reference(repo, &reference)?;
        return Ok(());
    }

    if let Ok(tag_obj) = repo.revparse_single(&format!("refs/tags/{}", rev)) {
        let commit = tag_obj
            .peel(ObjectType::Commit)?
            .into_commit()
            .map_err(|_| anyhow!("tag didn't peel to a commit"))?;
        repo.checkout_tree(commit.as_object(), None)?;
        repo.set_head_detached(commit.id())?;
        return Ok(());
    }

    let obj = repo
        .revparse_single(rev)
        .with_context(|| format!("rev not found: {}", rev))?;
    let commit = obj
        .peel(ObjectType::Commit)?
        .into_commit()
        .map_err(|_| anyhow!("rev didn't peel to a commit"))?;
    repo.checkout_tree(commit.as_object(), None)?;
    repo.set_head_detached(commit.id())?;
    Ok(())
}
