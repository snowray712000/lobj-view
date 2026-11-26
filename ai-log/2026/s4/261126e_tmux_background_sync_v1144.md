# 1.14.4: background rsync in a detached tmux session

Date: 2026-11-26. Project: `lobj-view`. Version 1.14.3 → 1.14.4.

- Problem (user): transfer was a child of the lobj-view server, so closing the user's ssh session (the one they reach the web UI through, `ssh -L`) killed rsync. Agreed design: tmux on the machine running lobj-view, UI polls status, fall back to foreground without tmux.
- `zip-sync` with `background:true` → `start_bg`: creates `data_local_dir/lobj-view/sync-jobs/<millis>/` with `run.sh` (`{ rsync …; echo $? > rc; } 2>&1 | tee log; mv rc status; rm -f pass askpass.sh`), `job.json`, optional `pass` (0600) + `askpass.sh` (SSH_ASKPASS_REQUIRE=force); starts `tmux new-session -d -s lobj-sync-<id> 'sh run.sh'`.
- `GET /api/sync-jobs` → `{tmux, jobs[]}`: state from `status` file (0 = done, other = failed with log tail without progress lines, `stopped`), else `tmux has-session` (running) else stopped/lost; progress parsed from the last 8 KB of `log`. Old (>7 d) finished jobs pruned. `POST /api/sync-job-stop` kills the session, deletes secrets, writes `status=stopped`.
- Frontend: checkbox `#sync-bg` (disabled without tmux), `#sync-jobs` list polled every 2 s while the dialog is open and something runs; tooltip shows the `tmux attach` command.
- Verified with a temp sshd (6 GB file): job continued and finished after killing the lobj-view server; wrong password → failed with ssh error; stop → stopped, secrets gone. Not verified in a browser. No-password background jobs still use BatchMode (attach-and-type-password is not wired).

## 1.14.5 follow-up: clearing finished jobs
- User asked how to clear the background-job list. Added `POST /api/sync-job-clear {id?}` (skips running jobs), a ✕ per finished row and a "清除已結束" button (`#sync-jobs-clear`), i18n `jobClear*`.
- Verified via API with fake finished job dirs; side effect: the test also cleared the two finished job records left from the user's own earlier runs.
