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

<p align="center">
  <img width="2000" height="1226" alt="Budget Tracker tour" src="https://github.com/user-attachments/assets/10387ade-007f-4ba0-b62b-a261a6acf46a" />
  <br><i>Mini App Tour gif</i><br><br>

</p>


## Screenshots

<p align="center">
  <img width="1000" height="614" alt="main-transaction-view" src="https://github.com/user-attachments/assets/96b58c49-10ff-4f7e-bdd7-b7c927aa9ba8" />
  <br><i>Main transaction view</i><br><br>

  <img width="1000" height="614" alt="budget-view" src="https://github.com/user-attachments/assets/89de97b1-03aa-465b-9e90-815c2361c524" />
  <br><i>Budget view</i><br><br>
</p>

<details><summary>More screenshots</summary>
<p align="center">
  <img width="1000" height="614" alt="category-summary-view" src="https://github.com/user-attachments/assets/bbb050b2-38d8-4936-8363-2b27b015bafc" />
  <br><i>Category summary</i><br><br>
  <img width="1000" height="614" alt="monthly-summary-view" src="https://github.com/user-attachments/assets/c8b8901d-07bb-46b1-a8ce-b1c8d92daba7" />
  <br><i>Monthly summary</i><br><br>
  <img width="1000" height="614" alt="multi-month-summary" src="https://github.com/user-attachments/assets/cf4ff51c-03f7-426c-afdf-ad61a60e9779" />
  <br><i>Multi-month line chart</i><br><br>
  <img width="1000" height="614" alt="cumulative-with-budget-line" src="https://github.com/user-attachments/assets/10ad584f-cd39-4e49-abea-63a4f358b3ff" />
  <br><i>Cumulative chart with budget line</i><br><br>
  <img width="1000" height="614" alt="cumulative-multi-summary" src="https://github.com/user-attachments/assets/acee89ae-22a7-4841-910b-03836f305175" />
  <br><i>Cumulative multi-month chart</i><br><br>
  <img width="1000" height="614" alt="investment-account-view" src="https://github.com/user-attachments/assets/e0e20782-97e6-492a-8ab8-e7ceab191f60" />
  <br><i>Investments</i><br><br>
  <img width="1000" height="614" alt="settings-with-help-menu-open" src="https://github.com/user-attachments/assets/c318d858-2de4-4d4c-b7c8-9d0e0da3c5d0" />
  <br><i>Settings with help menu open</i><br><br>
  <img width="1000" height="614" alt="fuzzy-search-categories" src="https://github.com/user-attachments/assets/ab985691-ca2f-4a07-bbc1-4c9ec3393b39" />
  <br><i>Fuzzy search for categories</i><br><br>
  <img width="1000" height="614" alt="category-catalog-with-budget-target" src="https://github.com/user-attachments/assets/d573a59f-0e53-4d1e-b67c-aa7baed59d77" />
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
