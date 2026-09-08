# User Guide

The companion to the [README](../README.md), with the longer explanations that don't belong there. The help bar at the bottom of every view shows the relevant keys, and `Ctrl+H` opens the full keybindings menu for whatever mode you're in.

## Common tasks

- [Add or edit a transaction](#adding-and-editing-transactions)
- [Find transactions by date or category](#filtering)
- [Set a monthly or category budget](#set-a-monthly-or-category-budget)
- [Forecast upcoming recurring payments](#forecasting-ahead)
- [Record an investment's latest value](#record-a-valuation)
- [Create a what-if ledger](#create-a-what-if-ledger)
- [Import or export transactions](#import-and-export)
- [Find your database or back up your data](#data-storage-and-backups)

## Contents

- [The main view](#the-main-view)
- [Adding and editing transactions](#adding-and-editing-transactions)
- [Filtering](#filtering)
- [Recurring transactions](#recurring-transactions)
- [Summary views](#summary-views)
- [Budgets](#budgets)
- [Investments](#investments)
- [The category catalog](#the-category-catalog)
- [Ledgers](#ledgers)
- [Import and export](#import-and-export)
- [Settings](#settings)
- [Data storage and backups](#data-storage-and-backups)
- [CSV format](#csv-format)

## The main view

The transaction list is where you land on launch, with a grand-total summary below it. The totals follow the active filter.

These are the keys for the main view. Some do different things in other views.

| Action | Key |
| --- | --- |
| Select a transaction | `↑` / `↓` |
| Jump by page | `PageUp` / `PageDown` |
| Jump to the first / last transaction | `Ctrl+↑` / `Ctrl+↓` |
| Sort by Date, Description, Category, Subcategory, Type, or Amount | `1`–`6` or `F1`–`F6`; press again to reverse |
| Add / edit / delete a transaction | `a` / `e` / `d` (delete asks for `y`/`n` confirmation) |
| Create a one-time copy dated today | `Ctrl+C` |
| Open the quick / advanced filter | `f` / `Ctrl+F` |
| Open recurring settings for the selected transaction | `r` |
| Open monthly summary / category summary / budgets | `s` / `c` / `b` |
| Open [investments](#investments) | `i` |
| Open [settings](#settings) | `o` |
| Clear an active filter, or quit when no filter is active | `q` / `Esc` |

## Adding and editing transactions

1. From the main view, press `a` to add a transaction, or select one and press `e` to edit it.
2. Fill in the fields, moving between them with `Tab` or `↑`/`↓`.
3. Move to Date, Description, or Amount and press `Enter` to save. `Esc` cancels.

New transactions start with today's date and type Expense. Enter a description and a positive amount; the type determines whether it's income or spending. Changing the type clears the category and subcategory.

`Ctrl+C` from the main view saves a copy immediately, dated today and without recurrence. Select the copy and press `e` if you need to change it.

| Field | Controls |
| --- | --- |
| Date | `=` or `→` moves forward a day; `-` or `←` moves back; `Shift+←` / `Shift+→` jump by month |
| Category and subcategory | `Enter` opens the picker. You can enable a searchable category picker in [settings](#settings) |
| Income/expense type | `←` / `→` or `Enter` toggle the type instead of saving |

## Filtering

### Quick filter

The quick filter (`f`) searches transaction descriptions as you type, ignoring case. `Enter` closes the input and keeps the filter applied; `Esc` or `Ctrl+R` clears it.

### Advanced filter

The advanced filter (`Ctrl+F`) combines date range, description, category, subcategory, type, recurring status, and amount range. Transactions must match all the criteria you fill in. `Tab` or `↑`/`↓` move between fields. `Enter` opens a picker on Category or Subcategory; on other fields, it applies the filter. `Ctrl+R` clears all filters.

Use `YYYY-MM-DD` for dates and plain decimal amounts. Range endpoints are included; blank bounds leave that end unrestricted. Invalid date or amount bounds are currently ignored, so check the resulting rows.

> **Current filter limitations:** `Esc` closes the form but keeps your edited criteria. They can take effect later, such as when you sort. After adding, editing, deleting, or copying transactions, reopen and apply the advanced filter before relying on its totals.

The Recurring field is a `←`/`→` toggle that cycles through blank (everything), `Recurring`, and `One-Time`. Setting it to `Recurring` narrows the table and the summary totals to your recurring payments and their generated occurrences, so you can see what a cycle costs.

## Recurring transactions

1. Select a transaction and press `r` to open recurring settings.
2. Use `←`/`→` on *Is Recurring* to set it to Yes.
3. Move to Frequency and press `Enter`, choose a frequency with `↑`/`↓`, then press `Enter` again.
4. Set an End Date if needed.
5. Move to *Is Recurring* or End Date and press `Enter` to save.

Available frequencies:

- Daily
- Weekly
- Bi-weekly
- Semi-monthly (15th and last day of the month)
- Semi-monthly, weekday adjusted (the same dates, moved back to Friday if they fall on a weekend)
- Monthly
- Quarterly
- Yearly

The original transaction's date is the start date. Occurrences are generated through today when the app starts or reloads transactions, and an optional end date stops the series.

> **Changing a series:** Generated transactions can't be edited or deleted separately. Pressing `e`, `d`, or `r` on one targets the original transaction. Deleting it removes the series, not just that occurrence. Clear your filter first if the original isn't visible.

### Forecasting ahead

By default generation stops at today. Set *Forecast Months Ahead* in settings and press `Enter` to save. This generates occurrences through today plus the chosen number of calendar months, up to 60. Forecast rows appear dimmed in the transaction table, the table title shows the active horizon, and the summary and budget views pick up the projected months so you can see where the year is heading.

Forecast occurrences are derived in memory like every other generated occurrence: nothing extra is written to the database, an end date still cuts the series off, and setting the horizon back to `0` stops generation beyond today. Future-dated transactions you've entered yourself remain.

## Summary views

### Monthly summary

**Open:** Press `s` from the main view.

The monthly summary shows a daily spending chart and monthly net-balance bars. The grand-total bar shows income, expenses, and net for the selected year. `↑`/`↓` move between months; `←`/`→`, `[`/`]`, or `PageUp`/`PageDown` move between years. Press `m` to compare months' spending by day of month.

`c` toggles cumulative spending. In single-month mode, a budget set for that month also appears as a guideline spread evenly across its days. Multi-month mode doesn't show budget guidelines.

### Category summary

**Open:** Press `c` from the main view.

The category summary shows income, expenses, and net by month, with category/subcategory rows underneath. `↑`/`↓` select rows; `Enter` on a month heading expands or collapses it. `PageUp`/`PageDown` jump between months, and `←`/`→` or `[`/`]` change years.

Both summary views use the filtered transactions. Press `q` or `Esc` to return to the main view.

To see how your spending compares to your budget, open the [budget view](#budgets).

## Budgets

**Open:** Press `b` from the main view.

The budget view compares spending against your monthly budget and any per-category budgets, and is where both are set. It opens on the current month even before you have entered anything, so you can set a budget on a brand new ledger.

| Action | Key |
| --- | --- |
| Select a category | `↑` / `↓` |
| Change month | `←` / `→` |
| Change year | `Shift+←` / `Shift+→` |
| Set the monthly budget | `t` |
| Set the selected category's budget | `e` |
| Open the category catalog | `c` |

### Set a monthly or category budget

1. Go to the month you want the change to start in.
2. Press `t` for the monthly budget, or select a category and press `e` for its budget.
3. Enter the amount. The popup starts with the budget already set for that month.
4. Use `↑`/`↓` to choose [how far the change reaches](#change-a-budget-over-time).
5. Press `Enter` to save, or `Esc` to cancel.

Only budgeted categories appear in the budget table. To budget a category that isn't listed, press `c` to open the [category catalog](#the-category-catalog), select the category, and press `b`. Budget changes made from the catalog start with the current month selected.

### Understand Total, Spare, and Over Allocated

Under a *Category Budgets* heading, the status panel totals every category budget you have set
(`Total`) and shows what is left over from the monthly budget (`Spare`), so you can see how much of
your budget is still unassigned without entering any transactions. `Spare` goes red and the status
reads *Over Allocated* when your category budgets add up to more than the monthly budget, unless actual spending already exceeds it. In that case, the status reads *Over Budget*.

### Change a budget over time

Budget changes are recorded by month. Using *From <month> on* leaves earlier months alone. Set your target to 2,000 in January and raise it to 2,500 in
March, and January and February still report against 2,000 while March onward uses 2,500.

Every place that edits a budget uses the same popup, so the choice of how far a change reaches is
always in front of you. The budget view is tied to the month you are looking at; the category
catalog has no month of its own, so it anchors on the current one. Reach for *Replace all months*
when you want an amount to apply to your whole history.

| Option | Effect |
| --- | --- |
| *From \<month\> on* | Carries the new amount forward until something else changes it |
| *\<month\> only* | Changes that one month and restores the previously scheduled amount from the next one |
| *Replace all months* | Discards the whole history for that budget and applies one amount to every month |
| *Remove \<month\> change* | Appears when a change starts in the selected month; deletes it so the month inherits the previous amount again |

> **This replaces the whole history:** *Replace all months* removes all changes for that budget, not just the one in the month you're looking at.

To correct an amount without losing later changes, go to the month that amount started and use
*From <month> on*. Use *Replace all months* only when you want one amount across the entire history,
including months where you previously set a different budget.

> **Clearing a budget:** Leave the amount empty, then choose the scope. *From <month> on* clears it until the next scheduled change; *<month> only* clears just that month; *Replace all months* clears its whole history. A category drops out of the table in months where it has no budget. *Remove <month> change* ignores the amount and removes that change instead.

## Investments

Press `i` from the main view to track what your investments are worth over time. Everything here is
entered by hand: there are no live prices, and no connection to your bank. You record what you see
when you check on an account, and the app works out the rest.

The idea is that you never type in a gain. You record two different things:

- a **valuation**, meaning "this account is worth $62,410 today"
- a **contribution** or **withdrawal**, meaning money crossing the account boundary

Growth is the change in value after accounting for money added or withdrawn. A contribution isn't counted as a dollar gain, though it can change the percentage shown for ROI.

### Getting started

Press `a` to add an account. Give it a name and, optionally, a type: it's free text, so use whatever
you actually call it (`Brokerage`, `Retirement (RRSP)`, `Crypto`).

If the account already exists in real life, fill in the opening position too. **Starting Value** is
what it's worth right now, and **Contributed So Far** is what you've put in, minus withdrawals.
If you leave the contributed amount blank, it defaults to the starting value, so opening gain is zero.
For example, a starting value of 1,000 with 800 contributed records 200 of existing gain. Zero is
valid too, for something you were given rather than bought. Leave both fields blank to start with
no opening entries.

Press `Enter` from a field other than Status to save the account. On Status, `Enter` toggles the status instead.

### Record a valuation

Press `v` from the accounts list or inside an account to open a valuation entry. It starts with
today's date and the cursor in Amount. Enter the value, change the date if needed, and press `Enter`
from Amount or Date to save. A valuation can be zero.

Use the account's end-of-day value, including any contributions or withdrawals dated that day.
The app treats same-day money movements as already included in the valuation, regardless of the
order you enter them.

> **One valuation per day:** If you record another valuation for the same account on the same date, it replaces the old one.

### Record a contribution or withdrawal

1. Select an account and press `Enter` to open it.
2. Press `a` to add an entry.
3. Use `←`/`→` in the *Entry* field to choose contribution or withdrawal.
4. Enter the date and a positive amount. The entry type determines the direction.
5. Press `Enter` from a field other than *Entry* to save. On *Entry*, `Enter` cycles the type instead.

Inside an account, you'll also find its full entry history, growth chart, and a year-by-year
breakdown of net contributions (contributions minus withdrawals) against gains. `a`, `e` and `d` act on whatever you're
looking at: accounts on the list, entries inside an account. The *Entry* field also lets you choose
a valuation.

### Read the growth chart

The chart compares total Value with Invested, meaning contributions minus withdrawals. Value above
Invested means a gain; below it means a loss. The y-axis starts at zero, so negative values fall
outside the plotted range.

### Change the chart's time range

`←`/`→` move the window through YTD, 1Y, 3Y, 5Y and All, but only the ranges that would actually
show you less than All are offered. Two months into tracking an account, `1Y` and `5Y` would both
just be All under a misleading label, so they're skipped until you have the history to fill them.
The date labels follow suit, showing days on a short window, months on a medium one, and years on a
long one.

### Understand Gain, ROI, and Annual

The panel above the chart shows three different measures:

- **Gain** is current value minus net contributions, including any gain recorded in the opening position.
- **ROI** is Gain divided by net contributions, as a percentage. It doesn't account for how long the money was invested and shows `N/A` when net contributions aren't positive.
- **Annual** estimates performance over the selected time range. It uses Modified Dietz returns, which weight money added or withdrawn by how long it was invested, then compounds the returns from each calendar-year segment. This is an approximation, not an exact time-weighted return independent of deposit timing.

Gain and ROI stay lifetime figures when you change the range; Annual follows the selected range.
For periods of at least a year, Annual converts the result to a yearly rate using 365.25 days per
year. For shorter periods, it shows the period return without extrapolating it.

Annual skips yearly segments it can't measure and shows `N/A` if there's no elapsed period, no
measurable segment, or no valid compounded result. Its requirements differ from ROI, so one can
show a number while the other shows `N/A`.

### Understand lifetime gain versus window gain

Pick any range other than All and the third column also shows what you gained *within that window*,
which is a different question from lifetime *Gain*: if you opened the account with a cost basis
lower than its value, that earlier growth counts towards *Gain* but was never watched happening, so
it belongs to no window. On All, that slot shows *As of*: the account's latest valuation date, or
the oldest of the included accounts' latest valuation dates in the portfolio. Accounts with no
valuation are left out of that date calculation; if none have one, it shows `-`.

Each account is measured from the window's start or its first entry, whichever is later. Opening a
new account today doesn't credit the window with gains it had already earned before you added it. Years run from the
previous New Year's Eve rather than from January 1, in both the YTD figure and the per-year table in
the account detail, so nothing recorded on New Year's Day falls between the two.

### Identify stale valuations

Each account's value starts from its latest valuation on or before today, then adds contributions
and subtracts withdrawals made after that date. Without a valuation, it uses net contributions.
The accounts table shows the latest valuation date so you can see how fresh that starting point is.

Valuations older than 30 days are flagged with a `!`, and the panel title counts how many are stale.
Accounts without valuations aren't counted as stale. It's a nudge to go and look, not an error.

### Archive an account

Accounts you've closed can be archived rather than deleted. Press `e` on one and flip its *Status*
field to `Archived`. Move to another field and press `Enter` to save.

Its entries stay stored, but while hidden it is excluded from the table, totals, and all historical
portfolio charts and returns. Press `A` (`Shift+A`) from the accounts list to include archived
accounts again, both on screen and in the calculations. You can then edit one to unarchive it.

Investments belong to the ledger they were created in, the same way transactions do, and are carried
along when you copy a ledger.

## The category catalog

The catalog holds your categories and subcategories. Open it from Settings (*Manage Categories*) or with `c` from the budget view. `q`/`Esc` clears an applied filter first. With no filter active, it returns to whichever view you came from.

- `↑`/`↓` move between entries, `PageUp`/`PageDown` jump by page, `Ctrl+↑`/`Ctrl+↓` jump to the first/last entry
- `f` filters the catalog as you type; while typing, `Enter` keeps it applied, and `Esc` or `Ctrl+R` clears it
- `a` adds a category, `e` or `Enter` edits the selected one, `d` deletes it
- `b` sets the selected category's budget, using the same popup as the budget view. The same popup
  is reachable from the *Budget* row inside the editor, which saves the category first so a brand
  new one has something to attach a budget to
- `1`-`5` (or `F1`-`F5`) sort by type, category, subcategory, tag, or budget; pressing the same key again flips the direction, and the sorted column is marked in the header
- Expense categories can optionally hold a budget, used by the budget view. Because the catalog has
  no month of its own, `b` dates the change from the current month, and the popup says so

## Ledgers

A ledger is a self-contained set of transactions. One database can hold several of them, so you
can keep separate books for different accounts or goals, or copy a ledger to experiment with
forecasting without touching your real data. Budget amounts are **per ledger**: each ledger keeps
its own monthly budget and its own category budgets, and copying a ledger copies its budgets with
it, so a scenario ledger can plan against a different budget without disturbing your real one.

> **Categories are shared:** Renaming or deleting a category affects transactions in every ledger. Deleting a category or changing it from Expense to Income also removes its budget history across all ledgers. Budget amount edits are separate per ledger; these catalog changes aren't.

Open the list from Settings (*Ledger*, which shows the ledger currently open).

- `↑`/`↓` move between ledgers; the open one is marked with a dot
- `Enter` switches to the selected ledger and returns to settings
- `a` adds an empty ledger, `e` renames the selected one
- `Ctrl+C` copies the selected ledger's transactions, budget history, investment accounts, and investment entries into a new ledger. Handy for trying a
  forecast or a what-if against real numbers without touching the original. You're offered a name
  like `Main (copy)`, which you can edit before saving.
- `d` deletes the selected ledger after a confirmation that names it and lists its transaction count, plus its investment-account count if any. The last remaining ledger can't be deleted.
- `q`/`Esc` returns to settings

> **Deleting a ledger:** This permanently deletes its transactions, budget history, investment accounts, and investment entries. Check the ledger name and counts before confirming with `y`.

The name of the open ledger is shown in the transaction list's title. The transaction list,
filters, summary views, budget view, and CSV import/export all apply to the open ledger only.

When upgrading a database from before ledger support, existing transactions are assigned to a ledger named `Main`.

To transfer transactions between ledgers, [export them to CSV, switch ledgers, then import that file](#import-and-export). Exporting doesn't remove the transactions from the original ledger.

### Create a what-if ledger

1. Open Settings (`o` from the main view), select *Ledger*, and press `Enter`.
2. Select the ledger you want to experiment with and press `Ctrl+C`.
3. Give the copy a name, like `What-if`, and press `Enter` to save it.
4. The copy is now selected. Press `Enter` again to switch to it and return to Settings.
5. Select *Forecast Months Ahead*, enter a whole number from `0` to `60`, and press `Enter` to save and return to the main view.
6. Press `s` for the monthly summary or `b` for budgets to review the [forecast](#forecasting-ahead). You can change the copy's budget amounts without changing the original's.

The copy has its own transactions and budgets, but still shares the category catalog. *Forecast Months Ahead* applies across the app, not just to the copy. Set it back to `0` and save to stop generating recurring occurrences beyond today. This doesn't hide or remove future-dated transactions you've entered yourself.

## Import and export

**Open:** Press `o` from the main view, then look under *Data Management*.

### Import transactions

1. Check that you're in the right ledger. Its name is in the transaction list's title.
2. Open Settings, select *Import Transactions (CSV)*, and press `Enter`.
3. Type or paste the full path to the CSV file, including its filename, and press `Enter` to import. See [CSV format](#csv-format) for columns and accepted values.

The path prompt is a text field, not a file browser. `Ctrl+U` clears it, `Ctrl+D` restores the default path, and `Esc` returns to Settings. Use a full path rather than `~` or environment variables, which aren't expanded.

Import adds rows unless their date, description, amount, type, category, and subcategory match a transaction already present. Matching rows are skipped, not updated, even if recurrence settings differ. This also skips identical rows within the file. Amounts such as `10` and `10.00` count as equal.

Generated recurring rows are skipped after parsing, since the app generates them from the original transactions. A parsing error stops the import before any rows are added. A successful import clears the filters and returns to the main view.

### Export transactions

1. Switch to the ledger you want to export.
2. Open Settings, select *Export Transactions (CSV)*, and press `Enter`.
3. Type or paste the full destination path, including the CSV filename, and press `Enter` to export. The path controls are the same as for import.

The export contains all transactions in the open ledger, regardless of the active filter, including generated occurrences through the current forecast horizon.

> **Existing files are overwritten:** Export replaces the destination file without asking. Use a new `.csv` filename unless you mean to replace an earlier export.

> **A CSV isn't a full backup:** It gives you a copy of your transactions, but leaves out investments, the category catalog, and budget history. To keep a copy of everything, see [Data storage and backups](#data-storage-and-backups).

## Settings

Press `o` to open settings. Use `Tab` or `↑`/`↓` to move between settings and type into number or path fields. For toggles, `←` sets No and `→` sets Yes.

Press `Enter` on a regular setting to save all preferences and return to the main view. On Ledger, Manage Categories, Import, or Export, it opens that action instead. `Esc` discards unsaved preference changes. Saving settings clears transaction filters.

The menu is grouped into sections:

**Data Management**

- *Database Path*: where the SQLite database lives (see [Data storage and backups](#data-storage-and-backups)).
- *Ledger*: shows the ledger currently open; opens the [ledger list](#ledgers) to switch or manage them.
- *Manage Categories*: opens the [category catalog](#the-category-catalog).
- *Import Transactions (CSV)*: adds transactions to the open ledger, skipping rows with matching core fields. See [Import transactions](#import-transactions).
- *Export Transactions (CSV)*: writes the open ledger's transactions to a CSV file for transfer or use elsewhere; see [Import and export](#import-and-export).

**Transaction View**

- *Hourly Rate*: optionally enter your hourly earning rate; a *Show Costs in Hours* toggle then appears that displays amounts as hours worked.

**Recurring Transactions**

- *Forecast Months Ahead*: projects recurring occurrences this many months past today (0-60). `0` stops at today.

**Input Preferences**

- *Fuzzy Search Categories*: opens a searchable `Category > Subcategory` list when choosing Category in the transaction form. Despite the setting's name, it matches text contained in the names, ignoring case, not typos. It doesn't change the separate Subcategory picker or advanced-filter pickers.

**General Preferences**

- *Hide Help Bar*: hides the bottom help bar if you want the extra screen space (`Ctrl+H` still works).

## Data storage and backups

Transactions, categories, and investments are stored together in a local SQLite database (`budget.db`). On first run with a new database, it's seeded with the default category catalog and a ledger named `Main`. Default locations:

- **Linux:** `$XDG_DATA_HOME/BudgetTracker/budget.db` (usually `~/.local/share/BudgetTracker/budget.db`)
- **macOS:** `~/Library/Application Support/BudgetTracker/budget.db`
- **Windows:** `%APPDATA%\BudgetTracker\budget.db`

Check *Database Path* in Settings for the actual location. Older configurations may put the database beside a previously configured CSV file instead. The setting takes a full filename, not just a folder, and the file doesn't have to be named `budget.db`.

When you save a new path, the app copies the current database there if the destination doesn't exist. If it already exists, the app switches to that database without merging them. The original file stays in place.

> **Using a cloud-synced folder:** This can carry the database between devices, but the app doesn't handle sync conflicts or simultaneous use. Close the app before switching devices, wait for syncing to finish, and keep separate backups.

App preferences live separately in a `config.json` in your OS config directory:

- **Linux:** `$XDG_CONFIG_HOME/BudgetTracker/config.json` (usually `~/.config/BudgetTracker/config.json`)
- **macOS:** `~/Library/Application Support/BudgetTracker/config.json`
- **Windows:** `%APPDATA%\BudgetTracker\config.json`

Changes are written to the database immediately as you add, edit, or delete, so there's no separate save step. CSV files are only written when you explicitly export.

### Back up your data

Note the database filename and location shown in Settings, close the app, then copy that file to wherever you keep your backups. Copy `config.json` too if you want to keep your app preferences. You'll need the database file for a full backup, not just a [CSV export](#export-transactions).

### Migrating from older versions

Older versions stored transactions in a CSV file. On the first migration check for each database,
the app looks for that file at its configured or default path and imports it if found. The same
[duplicate rules](#import-transactions) apply, and generated recurring rows are skipped. After
importing stored transaction rows, it tries to rename the original with `.migrated-backup` added
to the filename. If the rename fails, the original keeps its name. If the file wasn't present
during that first check, use manual CSV import later.

The database also has a schema version. Pending schema upgrades run in one database transaction,
so a failed schema upgrade rolls back its changes. The current app refuses database access and
shows an update-required error if the schema is newer than it supports. Keep app versions aligned
when sharing a database between devices.

The separate CSV import, backup rename, and migration marker aren't covered by the schema-upgrade
transaction. Keep a copy of your old CSV until you've checked the imported data.

## CSV format

Import and export use these columns:

```csv
date,description,amount,transaction_type,category,subcategory
```

Use these header names without spaces around the commas. Date, description, amount, and type are required. If the category column is omitted, it defaults to `Uncategorized`; an omitted subcategory defaults to empty.

- **Date:** accepts `YYYY-MM-DD`, `YYYY/MM/DD`, `DD/MM/YYYY`, or `DD-MM-YYYY`
- **Transaction type:** `Income` or `Expense`, case-insensitive; `i`/`e` also work
- **Category/Subcategory:** use names from the category catalog where possible. Import doesn't add catalog entries or enforce the transaction form's validation rules.

Exports also include `is_recurring`, `recurrence_frequency`, `recurrence_end_date`, and
`is_generated_from_recurring`. You can omit these columns when importing one-time transactions.
If boolean columns are present, use `true` or `false`, not blank cells.

For recurring transactions, set `is_recurring` to `true` and use one of these exact frequency values:

```text
Daily, Weekly, BiWeekly, SemiMonthly, SemiMonthlyWorkday, Monthly, Quarterly, Yearly
```

A recurring transaction without a frequency won't generate occurrences. The optional end date
accepts the same date formats listed above. Generated rows are parsed, then skipped and generated
again from their originals, so a malformed generated row can still stop an import.

Re-importing the same file won't add rows whose core fields already match, but it won't update
recurrence settings either. Export always writes the whole open ledger, not just filtered rows.
See [Import and export](#import-and-export) for the steps and overwrite warning.
