//! Form fields addressed by name instead of by index, in the same spirit as
//! [`crate::app::settings_types::SettingKey`]. Each form declares its fields once, and everything
//! keyed off a field (label, input handling, cursor, rendering) derives from that declaration
//! rather than from a separate list of indices that can drift out of alignment.

use std::marker::PhantomData;
use std::ops::{Index, IndexMut};

/// How a field behaves: what typing does to it, and what the arrow keys do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    Text,
    Date,
    Amount,
    /// Arrow keys cycle a fixed set of values.
    Toggle,
    /// Enter opens a picker.
    Selection,
}

impl FieldKind {
    pub const fn is_editable(self) -> bool {
        matches!(self, Self::Text | Self::Date | Self::Amount)
    }
}

/// The fields of one form, in the order they appear on screen.
pub trait FieldKey: Copy + Eq + 'static {
    const ALL: &'static [Self];

    fn index(self) -> usize;
    fn kind(self) -> FieldKind;
    fn label(self) -> &'static str;

    fn hint(self) -> &'static str {
        ""
    }
}

#[derive(Debug, Clone)]
pub struct FieldSet<K: FieldKey, const N: usize> {
    values: [String; N],
    focused: usize,
    key: PhantomData<K>,
}

impl<K: FieldKey, const N: usize> FieldSet<K, N> {
    /// Fails the build if `N` disagrees with the number of declared fields.
    const LENGTH_MATCHES_KEYS: () = assert!(
        N == K::ALL.len(),
        "FieldSet length must match the number of fields declared by its key enum"
    );

    pub fn new(values: [String; N]) -> Self {
        let () = Self::LENGTH_MATCHES_KEYS;
        Self {
            values,
            focused: 0,
            key: PhantomData,
        }
    }

    pub fn focused(&self) -> K {
        K::ALL[self.focused]
    }

    pub fn focus(&mut self, field: K) {
        self.focused = field.index();
    }

    pub fn focus_next(&mut self) {
        self.focused = (self.focused + 1) % N;
    }

    pub fn focus_previous(&mut self) {
        self.focused = if self.focused == 0 {
            N - 1
        } else {
            self.focused - 1
        };
    }

    pub fn focused_value(&self) -> &String {
        &self.values[self.focused]
    }

    pub fn iter(&self) -> impl Iterator<Item = (K, &String)> {
        K::ALL.iter().copied().zip(self.values.iter())
    }

    pub fn all_empty(&self) -> bool {
        self.values.iter().all(|value| value.is_empty())
    }

    /// Blanks every field and returns focus to the first one.
    pub fn reset(&mut self) {
        for value in self.values.iter_mut() {
            value.clear();
        }
        self.focused = 0;
    }
}

impl<K: FieldKey, const N: usize> Default for FieldSet<K, N> {
    fn default() -> Self {
        Self::new(std::array::from_fn(|_| String::new()))
    }
}

impl<K: FieldKey, const N: usize> Index<K> for FieldSet<K, N> {
    type Output = String;

    fn index(&self, field: K) -> &String {
        &self.values[field.index()]
    }
}

impl<K: FieldKey, const N: usize> IndexMut<K> for FieldSet<K, N> {
    fn index_mut(&mut self, field: K) -> &mut String {
        &mut self.values[field.index()]
    }
}

/// Declares a form's fields in one table. Variant order is screen order, and every arm is
/// required, so a new field cannot be added without giving it a kind and a label.
macro_rules! form_fields {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $($variant:ident => $kind:expr, $label:literal $(, $hint:literal)? ;)+
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        $vis enum $name {
            $($variant),+
        }

        impl crate::app::fields::FieldKey for $name {
            const ALL: &'static [Self] = &[$(Self::$variant),+];

            fn index(self) -> usize {
                self as usize
            }

            fn kind(self) -> crate::app::fields::FieldKind {
                match self {
                    $(Self::$variant => $kind),+
                }
            }

            fn label(self) -> &'static str {
                match self {
                    $(Self::$variant => $label),+
                }
            }

            fn hint(self) -> &'static str {
                match self {
                    $(Self::$variant => concat!("", $($hint)?)),+
                }
            }
        }
    };
}

form_fields! {
    pub enum AddEditField {
        Date => FieldKind::Date, "Date (YYYY-MM-DD)",
            "(◀/▶ or +/- for days, Shift+◀/▶ for months, Digits to enter)";
        Description => FieldKind::Text, "Description";
        Amount => FieldKind::Amount, "Amount";
        TransactionType => FieldKind::Toggle, "Type", "(◀/▶ or Enter to toggle)";
        Category => FieldKind::Selection, "Category", "(Enter to select)";
        Subcategory => FieldKind::Selection, "Subcategory", "(Enter to select)";
    }
}

form_fields! {
    pub enum AdvancedFilterField {
        DateFrom => FieldKind::Date, "Date From (YYYY-MM-DD)",
            "(◀/▶ or +/- days, Shift+◀/▶ months (jumps to today if empty), Digits to enter)";
        DateTo => FieldKind::Date, "Date To (YYYY-MM-DD)",
            "(◀/▶ or +/- days, Shift+◀/▶ months (jumps to today if empty), Digits to enter)";
        Description => FieldKind::Text, "Description";
        Category => FieldKind::Selection, "Category", "(Enter to select)";
        Subcategory => FieldKind::Selection, "Subcategory", "(Enter to select)";
        TransactionType => FieldKind::Toggle, "Type", "(◀/▶)";
        Recurring => FieldKind::Toggle, "Recurring", "(◀/▶)";
        AmountFrom => FieldKind::Amount, "Amount From";
        AmountTo => FieldKind::Amount, "Amount To";
    }
}

form_fields! {
    pub enum CategoryEditField {
        TransactionType => FieldKind::Toggle, "Transaction Type", "(Left/Right or Enter to toggle)";
        Category => FieldKind::Text, "Category";
        Subcategory => FieldKind::Text, "Subcategory", "(Optional)";
        Tag => FieldKind::Text, "Tag", "(Optional)";
        Budget => FieldKind::Selection, "Budget", "(Enter to set, expense only)";
    }
}

form_fields! {
    pub enum RecurringField {
        IsRecurring => FieldKind::Toggle, "Is Recurring", "(◀/▶ to toggle)";
        Frequency => FieldKind::Selection, "Frequency", "(Enter to select)";
        EndDate => FieldKind::Date, "End Date (YYYY-MM-DD)",
            "(Optional - ◀/▶ days, Shift+◀/▶ months, jumps to today if empty)";
    }
}

form_fields! {
    /// The opening position fields only apply when creating an account. Editing one leaves
    /// them inert, since by then its history is made of real entries you can edit directly.
    pub enum InvestmentAccountField {
        Name => FieldKind::Text, "Account Name";
        Kind => FieldKind::Text, "Type", "(Optional - e.g. Brokerage, Retirement (RRSP), Crypto)";
        Status => FieldKind::Toggle, "Status", "(◀/▶ to toggle)";
        OpeningValue => FieldKind::Amount, "Starting Value", "(Optional - what it is worth now)";
        OpeningInvested => FieldKind::Amount, "Contributed So Far",
            "(Optional - defaults to the starting value)";
        OpeningDate => FieldKind::Date, "As Of (YYYY-MM-DD)",
            "(◀/▶ or +/- for days, Shift+◀/▶ for months)";
    }
}

form_fields! {
    pub enum InvestmentEntryField {
        Date => FieldKind::Date, "Date (YYYY-MM-DD)",
            "(◀/▶ or +/- for days, Shift+◀/▶ for months, Digits to enter)";
        EntryKind => FieldKind::Toggle, "Entry", "(◀/▶ to toggle)";
        Amount => FieldKind::Amount, "Amount";
        Note => FieldKind::Text, "Note", "(Optional)";
    }
}

/// Which form a category picker was opened from. The two forms have separate field sets, so a
/// shared index could send the picked value back to the wrong one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectingField {
    AddEdit(AddEditField),
    AdvancedFilter(AdvancedFilterField),
}

impl SelectingField {
    pub fn add_edit(self) -> Option<AddEditField> {
        match self {
            Self::AddEdit(field) => Some(field),
            Self::AdvancedFilter(_) => None,
        }
    }

    pub fn advanced_filter(self) -> Option<AdvancedFilterField> {
        match self {
            Self::AdvancedFilter(field) => Some(field),
            Self::AddEdit(_) => None,
        }
    }
}
