# Command line

The `t-rex` client covers the same features as the web app. Commands read as `t-rex <noun> <verb>`:

```bash
t-rex login --server https://t-rex.example.com   # opens the browser, saves a token
t-rex whoami
t-rex project add "Website" --color "#0d9488"
t-rex absence add "Vacation"
t-rex entry add --start 09:00 --end 17:30 --break 45 --project Website --note "release"
t-rex entry add --start 09:00 --duration 7.5 --absence Vacation
t-rex entry list --from 2026-11-01 --to 2026-11-30   # sorted by date
t-rex entry list --sort hours --reverse              # longest days first
t-rex pending start                                  # work started now, the entry is added later
t-rex holiday import https://example.com/holidays.ics
t-rex settings set daily-hours 7.5
t-rex token list
t-rex logout
```

## Commands

| Command | Purpose |
|---------|---------|
| `login [--device NAME]` | Sign in through the browser and save a token |
| `logout` | Revoke this computer's token and forget it |
| `whoami` | Show the signed-in account and the server version |
| `project list` / `add NAME [--color HEX] [--archived]` / `edit PROJECT [--name] [--color] [--archive\|--unarchive]` / `remove PROJECT` | Projects, by name or id |
| `absence list` / `add NAME [--color HEX] [--unpaid]` / `edit ABSENCE [--name] [--color] [--paid\|--unpaid]` / `remove ABSENCE` | Absence kinds, by name or id |
| `entry list [--from DATE] [--to DATE] [--sort COLUMN] [--reverse]` | Time records of a period, by default the current month |
| `entry add` / `entry edit ID` | Give two of `--start`, `--end`, `--duration`; the third is worked out |
| `entry remove ID` | Delete a time record |
| `pending show` / `start [--time HH:MM]` / `clear` | The start of work that has no record yet |
| `holiday list [--year YEAR]` / `add DATE NAME` / `edit ID` / `remove ID` | Holidays |
| `holiday import URL` / `import-file PATH` | Import the all-day events of an `.ics` file |
| `settings show` / `settings set KEY VALUE` | Settings: `daily-hours`, `break`, `language`, `theme`, `show-email`, `carry-over` |
| `token list` / `remove ID` | Tokens of the account |

Time values accept several forms:

- **Clock times** (`--start`, `--end`, `--time`): `HH:MM`, 24-hour.
- **Durations** (`--duration`, `--break`): `7.5` or `7h` for hours, `450m` for minutes. A bare number of `--duration` is hours; a bare number of `--break` is minutes.
- **Signed hours** (carry-over): `-1:30`, `+12:00`.

`entry list` sorts with `--sort date` (default), `what`, `hours` or `id`. `--reverse` flips the order.

## Sign-in

`t-rex login` works like this:

1. The client listens on a random port of `127.0.0.1` and opens `<server>/auth` in the browser. If the browser does not open, the URL is printed.
2. After the normal sign-in, the server sends the browser back to the local port with a one-time code and the state the client sent.
3. The client checks the state and sends the code to `POST /api/v1/cli/token`, together with the device name.
4. The server returns a token, which the client saves.

The code expires after 2 minutes and works once. The client waits up to 5 minutes for the browser.

## Config file

The server address and token live in `config.toml` in the user's config folder:

| OS | Path |
|----|------|
| Linux | `~/.config/t-rex/config.toml` |
| macOS | `~/Library/Application Support/t-rex/config.toml` |
| Windows | `%APPDATA%\t-rex\config.toml` |

On Unix the file is readable only by its owner, because it holds the token. `T_REX_SERVER` or `--server` overrides the saved address.

## Tokens

- A token expires 10 days after it was created. After that, run `t-rex login` again.
- An account can hold at most 15 tokens. A login on a full account fails until an old token is removed (`t-rex token remove ID`, or in the web app).
- A token is named `<user>@<host>` by default. Change it with `t-rex login --device NAME`. If that name is already taken, the creation time is appended.
- `t-rex logout` revokes the token on the server. Revoking a token from `token remove` ends its access on that computer too.

## Shell completion

Completion covers commands, flags and fixed values, and project and absence names for `--project` and `--absence`. Add one line to your shell config. The completion asks the binary on each TAB, so it stays in step with the client:

```bash
# ~/.zshrc
source <(COMPLETE=zsh t-rex)

# ~/.bashrc
source <(COMPLETE=bash t-rex)
```

Start a new shell afterwards. Completion of project and absence names needs a saved server address and a token; without them, only commands and flags complete. The same mechanism works for fish, powershell and elvish (see `COMPLETE=<shell> t-rex` in the clap_complete docs).
