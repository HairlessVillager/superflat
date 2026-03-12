rm -rf temp/git
git init --bare temp/git
git --git-dir temp/git --work-tree temp/repo add . >/dev/null
git --git-dir temp/git --work-tree temp/repo commit -m "t0" >/dev/null
git --git-dir temp/git repack -a -d --depth 4095 --window 64
git --git-dir temp/git --work-tree temp/repo2 add . >/dev/null
git --git-dir temp/git --work-tree temp/repo2 commit -m "t1" >/dev/null
git --git-dir temp/git repack -a -d --depth 4095 --window 64
git --git-dir temp/git count-objects -v
git --git-dir temp/git --no-pager log --pretty=oneline
