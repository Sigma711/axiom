//! Source-backed corporate actions and their accounting effects.
//!
//! Historical raw-price stock simulations must apply a split before the first
//! observed session on or after its effective trading date. A split changes
//! units, not wealth: quantity scales by the split ratio, per-share cost basis
//! scales inversely, and realized P&L does not move.

use crate::data::StockAdjustmentEvidence;
use crate::types::Position;
use anyhow::{ensure, Result};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CorporateActionSource {
    pub provider: String,
    pub endpoint: String,
    pub fetched_at: DateTime<Utc>,
    pub issuer_confirmation_url: String,
    pub issuer_confirmation: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SourceBackedStockSplit {
    symbol: String,
    effective_at: DateTime<Utc>,
    effective_trading_date: NaiveDate,
    numerator: f64,
    denominator: f64,
    split_ratio: String,
    source: CorporateActionSource,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct StockSplitApplication {
    pub symbol: String,
    pub effective_trading_date: NaiveDate,
    pub size_before: f64,
    pub size_after: f64,
    pub average_entry_price_before: f64,
    pub average_entry_price_after: f64,
    pub cost_basis_before: f64,
    pub cost_basis_after: f64,
    pub realized_pnl_change: f64,
}

/// The price convention of the bars receiving corporate-action accounting.
/// Only proven raw prices may receive explicit split transformations; applying
/// them to split-adjusted prices would count the action twice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HistoricalPriceBasis {
    Raw,
    SplitAdjusted,
    Unverified,
}

impl SourceBackedStockSplit {
    pub fn from_evidence(symbol: &str, evidence: &StockAdjustmentEvidence) -> Result<Self> {
        let symbol = symbol.trim().to_ascii_uppercase();
        ensure!(!symbol.is_empty(), "stock split symbol must not be empty");
        ensure!(
            evidence.event.kind == "split",
            "corporate action is not a split"
        );
        ensure!(
            evidence.event.numerator.is_finite() && evidence.event.numerator > 0.0,
            "stock split numerator must be positive and finite"
        );
        ensure!(
            evidence.event.denominator.is_finite() && evidence.event.denominator > 0.0,
            "stock split denominator must be positive and finite"
        );
        let ratio = evidence.event.numerator / evidence.event.denominator;
        ensure!(
            ratio.is_finite() && ratio > 0.0,
            "stock split ratio must be positive and finite"
        );
        ensure!(
            !evidence.provider.trim().is_empty(),
            "stock split provider is missing"
        );
        ensure!(
            !evidence.endpoint.trim().is_empty(),
            "stock split endpoint is missing"
        );
        ensure!(
            !evidence.issuer_confirmation_url.trim().is_empty(),
            "stock split issuer confirmation is missing"
        );
        Ok(Self {
            symbol,
            effective_at: evidence.event.effective_at,
            effective_trading_date: evidence.event.effective_trading_date,
            numerator: evidence.event.numerator,
            denominator: evidence.event.denominator,
            split_ratio: evidence.event.split_ratio.clone(),
            source: CorporateActionSource {
                provider: evidence.provider.to_owned(),
                endpoint: evidence.endpoint.clone(),
                fetched_at: evidence.fetched_at,
                issuer_confirmation_url: evidence.issuer_confirmation_url.to_owned(),
                issuer_confirmation: evidence.issuer_confirmation.to_owned(),
            },
        })
    }

    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    pub fn effective_at(&self) -> DateTime<Utc> {
        self.effective_at
    }

    pub fn effective_trading_date(&self) -> NaiveDate {
        self.effective_trading_date
    }

    pub fn ratio(&self) -> f64 {
        self.numerator / self.denominator
    }

    pub fn split_ratio(&self) -> &str {
        &self.split_ratio
    }

    pub fn source(&self) -> &CorporateActionSource {
        &self.source
    }

    pub fn apply_to_position(&self, position: &mut Position) -> Result<StockSplitApplication> {
        ensure!(
            position.symbol.eq_ignore_ascii_case(&self.symbol),
            "stock split symbol does not match position"
        );
        ensure!(
            position.size.is_finite()
                && position.avg_entry_price.is_finite()
                && position.realized_pnl.is_finite(),
            "position accounting values must be finite"
        );
        let size_before = position.size;
        let average_entry_price_before = position.avg_entry_price;
        let realized_pnl_before = position.realized_pnl;
        let cost_basis_before = size_before.abs() * average_entry_price_before;
        let ratio = self.ratio();
        let size_after = size_before * ratio;
        let average_entry_price_after = if size_before.abs() >= 1e-9 {
            average_entry_price_before / ratio
        } else {
            average_entry_price_before
        };
        ensure!(
            size_after.is_finite() && average_entry_price_after.is_finite(),
            "stock split produced non-finite position accounting values"
        );
        position.size = size_after;
        position.avg_entry_price = average_entry_price_after;
        let cost_basis_after = position.size.abs() * position.avg_entry_price;
        Ok(StockSplitApplication {
            symbol: self.symbol.clone(),
            effective_trading_date: self.effective_trading_date,
            size_before,
            size_after: position.size,
            average_entry_price_before,
            average_entry_price_after: position.avg_entry_price,
            cost_basis_before,
            cost_basis_after,
            realized_pnl_change: position.realized_pnl - realized_pnl_before,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct StockSplitSchedule {
    events: Vec<SourceBackedStockSplit>,
}

impl StockSplitSchedule {
    pub fn new(
        mut events: Vec<SourceBackedStockSplit>,
        price_basis: HistoricalPriceBasis,
    ) -> Result<Self> {
        ensure!(
            price_basis == HistoricalPriceBasis::Raw,
            "explicit stock splits require a proven raw historical price basis"
        );
        events.sort_by(|left, right| {
            left.effective_trading_date
                .cmp(&right.effective_trading_date)
                .then_with(|| left.symbol.cmp(&right.symbol))
        });
        ensure!(
            events.windows(2).all(|pair| {
                pair[0].symbol != pair[1].symbol
                    || pair[0].effective_trading_date != pair[1].effective_trading_date
            }),
            "duplicate stock split for symbol and effective date"
        );
        Ok(Self { events })
    }

    pub fn empty() -> Self {
        Self { events: Vec::new() }
    }

    /// Events to apply before `current` is processed. With no previous
    /// observation, only an event on the current session date is due; this
    /// avoids replaying old splits against already adjusted starting data.
    pub fn due_between(
        &self,
        symbol: &str,
        previous: Option<DateTime<Utc>>,
        current: DateTime<Utc>,
    ) -> Vec<&SourceBackedStockSplit> {
        let symbol = symbol.trim();
        let current_date = current.date_naive();
        let previous_date = previous.map(|value| value.date_naive());
        self.events
            .iter()
            .filter(|event| event.symbol.eq_ignore_ascii_case(symbol))
            .filter(|event| event.effective_trading_date <= current_date)
            .filter(|event| match previous_date {
                Some(date) => event.effective_trading_date > date,
                None => event.effective_trading_date == current_date,
            })
            .collect()
    }

    pub fn events(&self) -> &[SourceBackedStockSplit] {
        &self.events
    }
}
