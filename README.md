# Budget Tracker TUI

<p align="center">
  <img src="budget_tracker_icon.png" alt="Budget Tracker Logo" width="150"/>
</p>

<p align="center">
  <a href="https://crates.io/crates/budget-tracker-tui"><img src="https://img.shields.io/crates/v/budget-tracker-tui" alt="Crates.io version"></a>
  <a href="https://crates.io/crates/budget-tracker-tui"><img src="https://img.shields.io/crates/d/budget-tracker-tui" alt="Crates.io downloads"></a>
  <a href="https://formulae.brew.sh/formula/budget-tracker"><img src="https://img.shields.io/homebrew/v/budget-tracker" alt="Homebrew version"></a>
  <a href="https://ratatui.rs/"><img src="https://ratatui.rs/built-with-ratatui/badge.svg" alt="Built With Ratatui"></a>
</p>

<p align="center">
  <sub>Previously published as <code>budget_tracker_tui</code>, which reached 221 downloads before the crate was renamed.</sub>
</p>

A terminal app for tracking your personal budget, built with [Rust](https://www.rust-lang.org) and [Ratatui](https://ratatui.rs).

```bash
cargo install budget-tracker-tui
# or
brew install budget-tracker
```

## Screenshots

<p align="center">
  <img width="1000" height="612" alt="main_view" src="https://github.com/user-attachments/assets/6aa9fa96-2918-4a0c-a7b2-b2b19d5eb27c" />
  <br><i>Main transaction view</i><br><br>

  <img width="1000" height="612" alt="cat_summary" src="https://github.com/user-attachments/assets/1b7f6741-8a05-4374-a66f-9fd099ffc9e9" />
  <br><i>Category summary</i><br><br>

  <img width="1000" height="612" alt="summary_view" src="https://github.com/user-attachments/assets/016d6227-3e3d-4a7f-ad3c-efdcba7dd068" />
  <br><i>Monthly summary with budget line</i><br><br>

  <img width="1009" height="593" alt="budget_view" src="https://github.com/user-attachments/assets/c5b25f94-8364-4f11-ba40-383a4e9bb850" />
  <br><i>Budget view</i><br><br>
</p>

<details><summary>More screenshots</summary>
<p align="center">
  <img width="1000" height="612" alt="summary_multi" src="https://github.com/user-attachments/assets/07011663-c1b7-45c5-b972-fb6c68bf432c" />
  <br><i>Multi-month line chart</i><br><br>
  <img width="1000" height="612" alt="summary_cumu" src="https://github.com/user-attachments/assets/5d99b5a9-2869-4a0a-aa63-a41cfb4a8787" />
  <br><i>Cumulative chart with budget line</i><br><br>
  <img width="1000" height="612" alt="summary_cum_multi" src="https://github.com/user-attachments/assets/97633bb7-fc4c-4c20-822f-2f7feb1e0065" />
  <br><i>Cumulative multi-month chart</i><br><br>
  <img width="1000" height="614" alt="investments" src="https://github.com/user-attachments/assets/290795dc-8c57-47e5-b1b9-1d52269b8990" />
  <br><i>Investments</i><br><br>
  <img width="1000" height="612" alt="help_menu" src="https://github.com/user-attachments/assets/7b994e22-1fc2-435d-9a07-e6c5373f51b4" />
  <br><i>Settings with help menu open</i><br><br>
  <img width="1000" height="612" alt="fuzzy_find" src="https://github.com/user-attachments/assets/9c6217f3-dbf5-47a9-8e86-b61c35449ba5" />
  <br><i>Fuzzy search for categories</i><br><br>
  <img width="1009" height="593" alt="category_catalog" src="https://github.com/user-attachments/assets/0f9430a9-50dc-4fec-ab2f-59ba21c331ca" />
  <br><i>Category catalog</i><br><br>
</p>
</details>

## Features

- Add, edit, delete, filter, and sort income and expense transactions
- Recurring transactions from daily to yearly, with optional forecasting
- Hierarchical categories and subcategories, editable in-app, with optional fuzzy search
- Monthly and category summaries with interactive charts
- Monthly and per-category budgets without changing past months
- Manual investment tracking for valuations, contributions, and growth
- Multiple ledgers for separate accounts or forecasts
- CSV import/export (duplicates skipped on import)
- Local SQLite storage with decimal arithmetic (no floating-point rounding errors)
- Fully keyboard-driven, with a built-in help menu
- Runs on Windows, macOS, and Linux; checks for new versions on startup

## Installation

### Cargo (Linux, macOS, Windows)

With Rust installed ([rustup.rs](https://rustup.rs)):

```bash
cargo install budget-tracker-tui
```

This puts the `budget-tracker` command on your PATH. If you installed the old `budget_tracker_tui` crate, uninstall it first with `cargo uninstall budget_tracker_tui`.

### Homebrew (macOS & Linux)

With Homebrew installed ([brew.sh](https://brew.sh)):

```bash
brew install budget-tracker
```

### Prebuilt binaries (no Rust required)

Grab the archive for your platform from the [Releases page](https://github.com/Feromond/budget-tracker-tui/releases), unpack it, and move `budget-tracker` onto your PATH. Linux has glibc and static musl builds for both x86_64 and arm64; the musl ones run on any distribution. Every release ships a `SHA256SUMS` file if you want to check the download:

```bash
sha256sum -c SHA256SUMS --ignore-missing
```

Windows also has an installer on that page. I don't have a Windows developer licence, so it shows as an unknown publisher.

### From source

```bash
git clone https://github.com/Feromond/budget-tracker-tui
cd budget-tracker-tui
cargo install --path .
```

## Usage

Launch with `budget-tracker`. The help bar at the bottom shows the keys for the current view, and `Ctrl+H` opens the full keybindings menu. Settings (`o`) is where you configure the database path, categories, CSV import/export, and display preferences. Budgets are set in the budget view (`b`).

For a more detailed walkthrough of every view and setting, see the [User Guide](docs/user-guide.md).

## Data & configuration

Transactions, categories, and investments live in a local SQLite database (`budget.db`), and app preferences in a `config.json`:

| OS      | Database                                       | Config                     |
| ------- | ---------------------------------------------- | -------------------------- |
| Linux   | `~/.local/share/BudgetTracker/`                | `~/.config/BudgetTracker/` |
| macOS   | `~/Library/Application Support/BudgetTracker/` | same                       |
| Windows | `%APPDATA%\BudgetTracker\`                     | same                       |

The database path is configurable in settings; point it at a cloud-synced folder (iCloud, Dropbox, etc.) to share your budget across devices. Changes are saved to the database immediately.

Older versions stored transactions in a `transactions.csv` file. On first launch, it is imported into the database automatically and renamed to `transactions.csv.migrated-backup`.

## CSV format

Import/export uses the columns `date, description, amount, transaction_type, category, subcategory`, with flexible date parsing. Import skips exact duplicates, so re-importing the same file is safe. Full details are in the [User Guide](docs/user-guide.md#csv-format).

## License

Licensed under the GNU General Public License v3.0. See [LICENSE](LICENSE) for details.
