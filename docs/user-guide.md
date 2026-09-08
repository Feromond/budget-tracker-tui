# User Guide

The companion to the [README](../README.md), with the longer explanations that don't belong there. You rarely need this while using the app, since the help bar at the bottom of every view shows the relevant keys and `Ctrl+H` opens the full keybindings menu for whatever mode you're in.

## The main view

The transaction list is where you land on launch, with a summary bar up top.

- `↑`/`↓` move between transactions, `PageUp`/`PageDown` jump by page, `Ctrl+↑`/`Ctrl+↓` jump to the first/last transaction
- `1`-`6` (or `F1`-`F6`) sort by Date, Description, Category, Subcategory, Type, or Amount; press again to reverse
- `a` adds a transaction, `e` edits the selected one, `d` deletes it (with a `y`/`n` confirmation), `Ctrl+C` copies it
- `f` opens the quick filter, `Ctrl+F` the advanced filter
- `r` opens recurring settings for the selected transaction
- `s`, `c`, and `b` open the monthly summary, category summary, and budget views
- `i` opens the [investments view](#investments)
- `o` opens settings
- `q` or `Esc` clears any active filter, or quits the app when no filter is active

## Adding and editing transactions

`Tab`/`Shift+Tab` or `↑`/`↓` move between fields and `Enter` saves. In the date field, `+` (or `=`) moves the date forward a day, `-` moves it back, and `Shift+←`/`Shift+→` jump by month. Category and subcategory fields offer a selection list (with fuzzy search if you've enabled it in settings), and `←`/`→` toggle the income/expense type.

## Filtering

The quick filter (`f`) matches as you type across your transactions. `Enter` closes the input and keeps the filter applied; `Esc` or `Ctrl+R` clears it.

The advanced filter (`Ctrl+F`) filters on multiple fields at once: date range, description, category, type, recurring status, and amount. `Tab` or `↑`/`↓` move between fields, `Enter` applies, `Esc` cancels, and `Ctrl+R` resets everything.

The Recurring field is a `←`/`→` toggle that cycles through blank (everything), `Recurring`, and `One-Time`. Setting it to `Recurring` narrows the table and the summary totals to your recurring payments and their generated occurrences, so you can see what a cycle costs.

## Recurring transactions

Select a transaction and press `r` to make it recurring. Available frequencies:

- Daily
- Weekly
- Bi-weekly
- Semi-monthly (1st and 15th)
- Semi-monthly, weekday adjusted (shifted off weekends)
- Monthly
- Quarterly
- Yearly

Occurrences are generated automatically from the start date up to today, and an optional end date stops the series. Generated occurrences stay linked to their source transaction. Edit or delete the source to affect the series.

### Forecasting ahead

By default generation stops at today. Set *Forecast Months Ahead* in settings to project occurrences further out, up to 60 months. Forecast rows appear dimmed in the transaction table, the table title shows the active horizon, and the summary and budget views pick up the projected months so you can see where the year is heading.

Forecast occurrences are derived in memory like every other generated occurrence: nothing extra is written to the database, an end date still cuts the series off, and setting the horizon back to `0` returns to today-only generation.

## Summary views

**Monthly summary (`s`)** shows income, expenses, and net per month with an interactive chart. `↑`/`↓` move between months, `←`/`→` (or `[`/`]`) move between years. `m` toggles a multi-month line chart, and `c` toggles cumulative mode, which also draws the monthly budget line for each month.

**Category summary (`c`)** breaks down each month by category. `Enter` expands or collapses a month, `PageUp`/`PageDown` move between months, `←`/`→` between years.

**Budget view (`b`)** compares spending against your monthly budget and any per-category budgets, and is where both are set. `↑`/`↓` move between categories, `←`/`→` between months, `Shift+←`/`Shift+→` between years. It opens on the current month even before you have entered anything, so you can set a budget on a brand new ledger.

Under a *Category Budgets* heading, the status panel totals every category budget you have set
(`Total`) and shows what is left over from the monthly budget (`Spare`), so you can see how much of
your budget is still unassigned without entering any transactions. `Spare` goes red and the status
reads *Over Allocated* when your category budgets add up to more than the monthly budget.

### Budgets change over time

Every budget is recorded with the month it starts applying, so changing one never rewrites what
earlier months were budgeted at. Set your target to 2,000 in January and raise it to 2,500 in
March, and January and February still report against 2,000 while March onward uses 2,500.

Every place that edits a budget uses the same popup, so the choice of how far a change reaches is
always in front of you. The budget view is tied to the month you are looking at; the category
catalog has no month of its own, so it anchors on the current one. Reach for *Replace all months*
when you want an amount to apply to your whole history.

Press `t` to set the monthly budget, and `e` to set the selected category's budget. Both start on
the amount currently in force, and `↑`/`↓` choose how far the change reaches:

- *From <month> on* carries the new amount forward until something else changes it
- *<month> only* changes that one month and restores the old amount from the next one
- *Replace all months* discards the whole history for that budget and applies one amount to every
  month
- *Remove <month> change* appears when a change starts in the selected month, and deletes it so the
  month inherits the previous amount again

*Replace all months* is the one to reach for when you got an amount wrong rather than when your
plan changed. Going back to your earliest month and using *from <month> on* would not do the same
thing, because any later change you made still overrides it from the month it starts.

`Enter` saves, `Esc` cancels, and an empty amount clears the budget from that month on, which also
drops the category from the table since only budgeted categories are listed. Press `c` to open the
[category catalog](#the-category-catalog), which is where you set a budget on a category that
doesn't have one yet.

## Investments

Press `i` from the main view to track what your investments are worth over time. Everything here is
entered by hand: there are no live prices, and no connection to your bank. You record what you see
when you check on an account, and the app works out the rest.

The idea is that you never type in a gain. You record two different things:

- a **valuation**, meaning "this account is worth $62,410 today"
- a **contribution** or **withdrawal**, meaning money crossing the account boundary

Growth is then whatever is left over: the change in value with your own money taken back out. Paying
into an account can never make your returns look better than they were.

### Getting started

Press `a` to add an account. Give it a name and, optionally, a type: it's free text, so use whatever
you actually call it (`Brokerage`, `Retirement (RRSP)`, `Crypto`).

If the account already exists in real life, fill in the opening position too. **Starting Value** is
what it's worth right now, and **Contributed So Far** is how much of that you put in yourself. They
default to the same number, which is right if you're starting fresh. Set the contributed amount
lower when part of the balance is growth you earned before you started tracking, otherwise the app
will report your entire existing balance as a gain on day one. Zero is a valid answer too, for
something you were given rather than bought.

### Day to day

`v` records a valuation, dated today, with the cursor already in the amount. It works from the
accounts list or from inside an account, and it's the one to press whenever you check on your
investments.

`Enter` opens the selected account, showing its full entry history, its own growth chart, and a
year-by-year breakdown of what you contributed against what it earned. `a`, `e` and `d` always act
on whatever the rows on screen are: accounts on the list, entries inside an account. So `a` from
inside an account adds an entry, and the *Entry* field (`←`/`→`) switches it between contribution,
withdrawal and valuation.

Recording a second valuation for an account on a date it already has one replaces it, since two
different values for the same day don't mean anything.

### Reading the numbers

The chart draws your total value above your cumulative contributions. The vertical gap between the
two lines is your gain at that moment, so growth is the gap widening. The y-axis always starts at
zero, which is what keeps that gap readable as a proportion.

`←`/`→` move the window through YTD, 1Y, 3Y, 5Y and All, but only the ranges that would actually
show you less than All are offered. Two months into tracking an account, `1Y` and `5Y` would both
just be All under a misleading label, so they're skipped until you have the history to fill them.
The date labels follow suit, showing days on a short window, months on a medium one, and years on a
long one.

The panel above it puts the dollar figure first, because it's the one that isn't open to
interpretation, then two rates that answer different questions:

- **ROI** is simply your gain over what you put in. It's the number most people mean, but it's
  distorted by *when* you contributed: money added last month has barely had a chance to grow
- **Annual** is a time-weighted annualized return, which chains each year's performance and weights
  contributions by how long they were actually invested. It's closer to "how well did these
  investments do", independent of your deposit timing

Both read `N/A` rather than guessing when there's nothing invested to measure against. Over periods
shorter than a year, *Annual* reports the period return as-is instead of extrapolating a few weeks
out to twelve months.

Pick any range other than All and the third column also shows what you gained *within that window*,
which is a different question from lifetime *Gain*: if you opened the account with a cost basis
lower than its value, that earlier growth counts towards *Gain* but was never watched happening, so
it belongs to no window. On All, that slot shows your oldest valuation date instead.

Every window measures each account from its own first entry, so opening a new account today doesn't
credit the portfolio with whatever it had already earned before you added it. Years run from the
previous New Year's Eve rather than from January 1, in both the YTD figure and the per-year table in
the account detail, so nothing recorded on New Year's Day falls between the two.

### As-of dates and staleness

Your portfolio total is a sum of valuations taken on different days, so the accounts table shows the
date of each one. Anything older than 30 days is flagged with a `!` and the panel title counts how
many are stale. It's a nudge to go and look, not an error.

### Archiving

Accounts you've closed can be archived rather than deleted. Press `e` on one and flip its *Status*
field to `Archived`. Its history stays intact so past performance stays correct, but it drops out of
the table and the totals. `A` (`Shift+A`) toggles archived accounts back into view so you can read
them or unarchive one.

Investments belong to the ledger they were created in, the same way transactions do, and are carried
along when you copy a ledger.

## The category catalog

The catalog holds your categories and subcategories. Open it from Settings (*Manage Categories*) or with `c` from the budget view. `q`/`Esc` returns to whichever view you came from.

- `↑`/`↓` move between entries, `PageUp`/`PageDown` jump by page, `Ctrl+↑`/`Ctrl+↓` jump to the first/last entry
- `f` filters the catalog as you type; `Enter` keeps the filter applied, `Esc` or `Ctrl+R` clears it
- `a` adds a category, `e` or `Enter` edits the selected one, `d` deletes it
- `b` sets the selected category's budget, using the same popup as the budget view. The same popup
  is reachable from the *Budget* row inside the editor, which saves the category first so a brand
  new one has something to attach a budget to
- `1`-`5` (or `F1`-`F5`) sort by type, category, subcategory, tag, or budget; pressing the same key again flips the direction, and the sorted column is marked in the header
- Expense categories can optionally hold a budget, used by the budget view. Because the catalog has
  no month of its own, `b` dates the change from the current month, and the popup says so

## Ledgers

A ledger is a self-contained set of transactions. One database can hold several of them, so you
can keep separate books for different accounts or goals, or copy of a scenario to experiment with
forecasting without touching your real data. Every ledger in a database **shares the same category
catalog**, so a category you rename or delete updates the transactions in all of them. Budget
amounts are **per ledger**, though: each ledger keeps its own monthly budget and its own category
budgets, and copying a ledger copies its budgets with it, so a scenario ledger can plan against a
different budget without disturbing your real one.

Open the list from Settings (*Ledger*, which shows the ledger currently open).

- `↑`/`↓` move between ledgers; the open one is marked with a dot
- `Enter` switches to the selected ledger and returns to settings
- `a` adds an empty ledger, `e` renames the selected one
- `Ctrl+C` copies the selected ledger, transactions and all, into a new one. Handy for trying a
  forecast or a what-if against real numbers without touching the original. You're offered a name
  like `Main (copy)`, which you can edit before saving.
- `d` deletes the selected ledger **and every transaction in it**, after a confirmation that names
  the ledger and its transaction count. The last remaining ledger can't be deleted.
- `q`/`Esc` returns to settings

The name of the open ledger is shown in the transaction list's title. The transaction list,
filters, summary views, budget view, and CSV import/export all apply to the open ledger only.

Existing databases upgrade automatically: all of your current transactions land in a ledger named
`Main`, and nothing else changes.

To move transactions between ledgers, export them to CSV, switch ledgers, then import that file.

## Settings

Press `o` to open settings. The menu is grouped into sections:

**Data Management**

- *Database Path*: where the SQLite database lives (see [Data storage](#data-storage) below).
- *Ledger*: shows the ledger currently open; opens the [ledger list](#ledgers) to switch or manage them.
- *Manage Categories*: opens the [category catalog](#the-category-catalog).
- *Import Transactions (CSV)*: merges a CSV file into the open ledger; new rows are added, exact duplicates are skipped.
- *Export Transactions (CSV)*: writes the open ledger's transactions to a CSV file for backup or use elsewhere.

**Transaction View**

- *Hourly Rate*: optionally enter your hourly earning rate; a *Show Costs in Hours* toggle then appears that displays amounts as hours worked.

**Recurring Transactions**

- *Forecast Months Ahead*: projects recurring occurrences this many months past today (0-60). `0` stops at today.

**Input Preferences**

- *Fuzzy Search Categories*: enables fuzzy matching when picking categories and subcategories.

**General Preferences**

- *Hide Help Bar*: hides the bottom help bar if you want the extra screen space (`Ctrl+H` still works).

## Data storage

Transactions, categories, and investments are stored together in a local SQLite database (`budget.db`). On first run with a new database, it's seeded with the default category catalog and a ledger named `Main`. Default locations:

- **Linux:** `$XDG_DATA_HOME/BudgetTracker/budget.db` (usually `~/.local/share/BudgetTracker/budget.db`)
- **macOS:** `~/Library/Application Support/BudgetTracker/budget.db`
- **Windows:** `%APPDATA%\BudgetTracker\budget.db`

The path is configurable in settings. Point it at a folder synced by iCloud, Google Drive, Dropbox, OneDrive, or similar to sync your budget across devices.

App preferences live separately in a `config.json` in your OS config directory:

- **Linux:** `~/.config/BudgetTracker/config.json`
- **macOS:** `~/Library/Application Support/BudgetTracker/config.json`
- **Windows:** `%APPDATA%\BudgetTracker\config.json`

Changes are written to the database immediately as you add, edit, or delete, so there's no separate save step. CSV files are only written when you explicitly export.

### Migrating from older versions

Versions before 1.4.0 stored transactions in a `transactions.csv` file. On first launch, the app imports that file into the database automatically and renames the original to `transactions.csv.migrated-backup`. Nothing is deleted.

The database carries a data version, and the app upgrades it in place when you install a newer release. Upgrades run in a single transaction, so a failure leaves the database exactly as it was. If you open a database that a **newer** version of the app has already upgraded (easy to do when the file is synced between machines running different versions), the older app refuses to open it and tells you to update, rather than reading or writing a schema it doesn't understand.

## CSV format

Import and export use these columns:

```
date, description, amount, transaction_type, category, subcategory
```

- **Date:** accepts `YYYY-MM-DD`, `YYYY/MM/DD`, `DD/MM/YYYY`, or `DD-MM-YYYY`
- **Transaction type:** `Income` or `Expense`, case-insensitive; `i`/`e` also work
- **Category/Subcategory:** should reference categories that exist in the category catalog (manageable in settings)

Exports additionally include the recurring columns (`is_recurring, recurrence_frequency, recurrence_end_date, is_generated_from_recurring`). These are optional on import and default to a non-recurring transaction; generated recurring rows in a file are ignored on import and re-derived from their source transaction instead.

Importing merges into the database and skips exact duplicates, so re-importing the same file is safe. Exporting writes the full set you see in the app, including generated recurring occurrences.
