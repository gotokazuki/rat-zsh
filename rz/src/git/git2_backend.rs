use anyhow::{Context, Result};
use git2::{Cred, FetchOptions, RemoteCallbacks, Repository, ResetType};

pub fn ensure_repo(url: &str, dest: &std::path::Path, rev: Option<&str>) -> Result<()> {
    if dest.join(".git").exists() {
        let repo = Repository::open(dest)?;
        fetch_origin(&repo)?;
        if let Some(r) = rev {
            checkout_rev(&repo, r)?;
        } else {
            let head = repo.head()?.peel_to_commit()?;
            repo.reset(head.as_object(), ResetType::Hard, None)?;
        }
        Ok(())
    } else {
        let repo = Repository::clone(url, dest).with_context(|| format!("git clone {}", url))?;
        if let Some(r) = rev {
            checkout_rev(&repo, r)?;
        }
        Ok(())
    }
}

pub fn fetch_origin(repo: &Repository) -> Result<()> {
    let mut cb = RemoteCallbacks::new();
    cb.credentials(|_url, username_from_url, _allowed| {
        Cred::ssh_key_from_agent(username_from_url.unwrap_or("git")).or_else(|_| Cred::default())
    });

    let mut fo = FetchOptions::new();
    fo.remote_callbacks(cb);

    let mut remote = repo
        .find_remote("origin")
        .or_else(|_| repo.remote_anonymous(repo.find_remote("origin")?.url().unwrap_or("")))?;
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
