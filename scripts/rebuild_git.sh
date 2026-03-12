rm -rf temp/git
git init --bare temp/git

git --git-dir temp/git --work-tree temp/repo add . >/dev/null
git --git-dir temp/git --work-tree temp/repo commit -m "t0" >/dev/null
time git --git-dir temp/git repack -d
git --git-dir temp/git count-objects -vH

git --git-dir temp/git --work-tree temp/repo2 add . >/dev/null
git --git-dir temp/git --work-tree temp/repo2 commit -m "t1" >/dev/null
time git --git-dir temp/git repack -d
git --git-dir temp/git count-objects -vH

git --git-dir temp/git --no-pager log --pretty=oneline
