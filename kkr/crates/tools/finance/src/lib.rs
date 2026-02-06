use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use std::env;
use std::time::Duration;

use kkr_core::tool::{Tool, ToolCategory, ToolContext, ToolMetadata, ToolSchema};
use kkr_core::Result;

const DEFAULT_TIMEOUT_SECS: u64 = 30;

pub struct YahooQuoteTool {
    client: Client,
}

impl Default for YahooQuoteTool {
    fn default() -> Self {
        Self::new()
    }
}

impl YahooQuoteTool {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }
}

#[derive(Debug, Deserialize)]
struct QuoteParams {
    symbols: Vec<String>,
}

#[async_trait]
impl Tool for YahooQuoteTool {
    fn name(&self) -> &str {
        "yahoo_quote"
    }

    fn description(&self) -> &str {
        "Get real-time stock quotes from Yahoo Finance. Returns price, change, volume, and market cap."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "symbols": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Stock symbols (e.g., ['AAPL', 'GOOGL', 'MSFT'])"
                }
            }),
            required: vec!["symbols".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Search)
            .with_tags(vec!["finance", "stocks", "yahoo", "quote", "market"])
            .with_read_only(true)
            .with_priority(75)
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: QuoteParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::tool(format!("Invalid parameters: {}", e)))?;

        let symbols = params.symbols.join(",");
        let url = format!(
            "https://query1.finance.yahoo.com/v7/finance/quote?symbols={}",
            symbols
        );

        let response = self
            .client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0")
            .send()
            .await
            .map_err(|e| kkr_core::Error::tool(format!("Request failed: {}", e)))?;

        let status = response.status();
        let body: Value = response
            .json()
            .await
            .map_err(|e| kkr_core::Error::tool(format!("Failed to parse response: {}", e)))?;

        if !status.is_success() {
            return Err(kkr_core::Error::tool(format!("Yahoo API error: {}", body)));
        }

        let quotes = body["quoteResponse"]["result"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|q| {
                        json!({
                            "symbol": q.get("symbol"),
                            "name": q.get("shortName"),
                            "price": q.get("regularMarketPrice"),
                            "change": q.get("regularMarketChange"),
                            "change_percent": q.get("regularMarketChangePercent"),
                            "volume": q.get("regularMarketVolume"),
                            "market_cap": q.get("marketCap"),
                            "pe_ratio": q.get("trailingPE"),
                            "52w_high": q.get("fiftyTwoWeekHigh"),
                            "52w_low": q.get("fiftyTwoWeekLow"),
                            "currency": q.get("currency"),
                            "exchange": q.get("exchange")
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Ok(json!({
            "quotes": quotes,
            "count": quotes.len()
        }))
    }
}

pub struct YahooHistoryTool {
    client: Client,
}

impl Default for YahooHistoryTool {
    fn default() -> Self {
        Self::new()
    }
}

impl YahooHistoryTool {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }
}

#[derive(Debug, Deserialize)]
struct HistoryParams {
    symbol: String,
    #[serde(default = "default_range")]
    range: String,
    #[serde(default = "default_interval")]
    interval: String,
}

fn default_range() -> String {
    "1mo".to_string()
}

fn default_interval() -> String {
    "1d".to_string()
}

#[async_trait]
impl Tool for YahooHistoryTool {
    fn name(&self) -> &str {
        "yahoo_history"
    }

    fn description(&self) -> &str {
        "Get historical price data for a stock. Supports various ranges and intervals."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "symbol": {
                    "type": "string",
                    "description": "Stock symbol (e.g., 'AAPL')"
                },
                "range": {
                    "type": "string",
                    "description": "Time range: 1d, 5d, 1mo, 3mo, 6mo, 1y, 2y, 5y, 10y, ytd, max",
                    "default": "1mo"
                },
                "interval": {
                    "type": "string",
                    "description": "Data interval: 1m, 5m, 15m, 30m, 1h, 1d, 1wk, 1mo",
                    "default": "1d"
                }
            }),
            required: vec!["symbol".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Search)
            .with_tags(vec!["finance", "stocks", "history", "chart", "prices"])
            .with_read_only(true)
            .with_priority(70)
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: HistoryParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::tool(format!("Invalid parameters: {}", e)))?;

        let url = format!(
            "https://query1.finance.yahoo.com/v8/finance/chart/{}?range={}&interval={}",
            params.symbol, params.range, params.interval
        );

        let response = self
            .client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0")
            .send()
            .await
            .map_err(|e| kkr_core::Error::tool(format!("Request failed: {}", e)))?;

        let status = response.status();
        let body: Value = response
            .json()
            .await
            .map_err(|e| kkr_core::Error::tool(format!("Failed to parse response: {}", e)))?;

        if !status.is_success() {
            return Err(kkr_core::Error::tool(format!("Yahoo API error: {}", body)));
        }

        let result = &body["chart"]["result"][0];
        let meta = &result["meta"];
        let timestamps = result["timestamp"].as_array();
        let quotes = &result["indicators"]["quote"][0];

        let data_points: Vec<Value> = timestamps
            .map(|ts| {
                let opens = quotes["open"].as_array();
                let highs = quotes["high"].as_array();
                let lows = quotes["low"].as_array();
                let closes = quotes["close"].as_array();
                let volumes = quotes["volume"].as_array();

                ts.iter()
                    .enumerate()
                    .map(|(i, t)| {
                        json!({
                            "timestamp": t,
                            "open": opens.and_then(|a| a.get(i)),
                            "high": highs.and_then(|a| a.get(i)),
                            "low": lows.and_then(|a| a.get(i)),
                            "close": closes.and_then(|a| a.get(i)),
                            "volume": volumes.and_then(|a| a.get(i))
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(json!({
            "symbol": params.symbol,
            "currency": meta.get("currency"),
            "exchange": meta.get("exchangeName"),
            "range": params.range,
            "interval": params.interval,
            "data": data_points,
            "count": data_points.len()
        }))
    }
}

pub struct CryptoQuoteTool {
    client: Client,
}

impl Default for CryptoQuoteTool {
    fn default() -> Self {
        Self::new()
    }
}

impl CryptoQuoteTool {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }
}

#[derive(Debug, Deserialize)]
struct CryptoParams {
    symbols: Vec<String>,
    #[serde(default = "default_currency")]
    vs_currency: String,
}

fn default_currency() -> String {
    "usd".to_string()
}

#[async_trait]
impl Tool for CryptoQuoteTool {
    fn name(&self) -> &str {
        "crypto_quote"
    }

    fn description(&self) -> &str {
        "Get cryptocurrency prices from CoinGecko API. Free tier, no API key required."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "symbols": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Crypto IDs (e.g., ['bitcoin', 'ethereum', 'solana'])"
                },
                "vs_currency": {
                    "type": "string",
                    "description": "Target currency (default: usd)",
                    "default": "usd"
                }
            }),
            required: vec!["symbols".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Search)
            .with_tags(vec!["crypto", "bitcoin", "ethereum", "prices", "coingecko"])
            .with_read_only(true)
            .with_priority(75)
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: CryptoParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::tool(format!("Invalid parameters: {}", e)))?;

        let ids = params.symbols.join(",");
        let url = format!(
            "https://api.coingecko.com/api/v3/coins/markets?vs_currency={}&ids={}&order=market_cap_desc&sparkline=false&price_change_percentage=24h,7d",
            params.vs_currency, ids
        );

        let response = self
            .client
            .get(&url)
            .header("User-Agent", "KKR/1.0")
            .send()
            .await
            .map_err(|e| kkr_core::Error::tool(format!("Request failed: {}", e)))?;

        let status = response.status();
        let body: Value = response
            .json()
            .await
            .map_err(|e| kkr_core::Error::tool(format!("Failed to parse response: {}", e)))?;

        if !status.is_success() {
            return Err(kkr_core::Error::tool(format!("CoinGecko API error: {}", body)));
        }

        let quotes: Vec<Value> = body
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|c| {
                        json!({
                            "id": c.get("id"),
                            "symbol": c.get("symbol"),
                            "name": c.get("name"),
                            "price": c.get("current_price"),
                            "market_cap": c.get("market_cap"),
                            "market_cap_rank": c.get("market_cap_rank"),
                            "volume_24h": c.get("total_volume"),
                            "change_24h": c.get("price_change_percentage_24h"),
                            "change_7d": c.get("price_change_percentage_7d_in_currency"),
                            "high_24h": c.get("high_24h"),
                            "low_24h": c.get("low_24h"),
                            "ath": c.get("ath"),
                            "ath_change_percent": c.get("ath_change_percentage"),
                            "circulating_supply": c.get("circulating_supply"),
                            "total_supply": c.get("total_supply")
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(json!({
            "currency": params.vs_currency,
            "quotes": quotes,
            "count": quotes.len()
        }))
    }
}

pub struct AlphaVantageQuoteTool {
    client: Client,
    api_key: Option<String>,
}

impl Default for AlphaVantageQuoteTool {
    fn default() -> Self {
        Self::new()
    }
}

impl AlphaVantageQuoteTool {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("Failed to create HTTP client"),
            api_key: env::var("ALPHAVANTAGE_API_KEY").ok(),
        }
    }

    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }
}

#[derive(Debug, Deserialize)]
struct AlphaVantageParams {
    symbol: String,
    function: Option<String>,
}

#[async_trait]
impl Tool for AlphaVantageQuoteTool {
    fn name(&self) -> &str {
        "alphavantage_quote"
    }

    fn description(&self) -> &str {
        "Get stock data from Alpha Vantage API. Supports real-time quotes and fundamentals."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "symbol": {
                    "type": "string",
                    "description": "Stock symbol (e.g., 'AAPL')"
                },
                "function": {
                    "type": "string",
                    "description": "API function: GLOBAL_QUOTE, OVERVIEW, TIME_SERIES_DAILY",
                    "default": "GLOBAL_QUOTE"
                }
            }),
            required: vec!["symbol".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Search)
            .with_tags(vec!["finance", "stocks", "alphavantage", "fundamentals"])
            .with_read_only(true)
            .with_priority(70)
            .with_requires(vec!["ALPHAVANTAGE_API_KEY"])
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let env_key = env::var("ALPHAVANTAGE_API_KEY").ok();
        let api_key = self
            .api_key
            .as_ref()
            .or(env_key.as_ref())
            .ok_or_else(|| kkr_core::Error::tool("ALPHAVANTAGE_API_KEY not set".to_string()))?;

        let params: AlphaVantageParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::tool(format!("Invalid parameters: {}", e)))?;

        let function = params.function.unwrap_or_else(|| "GLOBAL_QUOTE".to_string());

        let url = format!(
            "https://www.alphavantage.co/query?function={}&symbol={}&apikey={}",
            function, params.symbol, api_key
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| kkr_core::Error::tool(format!("Request failed: {}", e)))?;

        let status = response.status();
        let body: Value = response
            .json()
            .await
            .map_err(|e| kkr_core::Error::tool(format!("Failed to parse response: {}", e)))?;

        if !status.is_success() {
            return Err(kkr_core::Error::tool(format!("Alpha Vantage API error: {}", body)));
        }

        if body.get("Error Message").is_some() {
            return Err(kkr_core::Error::tool(format!("Alpha Vantage error: {}", body["Error Message"])));
        }

        Ok(json!({
            "symbol": params.symbol,
            "function": function,
            "data": body
        }))
    }
}

pub struct ForexQuoteTool {
    client: Client,
}

impl Default for ForexQuoteTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ForexQuoteTool {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }
}

#[derive(Debug, Deserialize)]
struct ForexParams {
    base: String,
    #[serde(default)]
    symbols: Option<Vec<String>>,
}

#[async_trait]
impl Tool for ForexQuoteTool {
    fn name(&self) -> &str {
        "forex_rates"
    }

    fn description(&self) -> &str {
        "Get foreign exchange rates. Free API, no key required."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "base": {
                    "type": "string",
                    "description": "Base currency code (e.g., 'USD', 'EUR')"
                },
                "symbols": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Target currencies (e.g., ['EUR', 'GBP', 'JPY']). If empty, returns all."
                }
            }),
            required: vec!["base".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Search)
            .with_tags(vec!["forex", "currency", "exchange", "rates"])
            .with_read_only(true)
            .with_priority(70)
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: ForexParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::tool(format!("Invalid parameters: {}", e)))?;

        let symbols_param = params
            .symbols
            .as_ref()
            .map(|s| format!("&symbols={}", s.join(",")))
            .unwrap_or_default();

        let url = format!(
            "https://api.frankfurter.app/latest?from={}{}",
            params.base, symbols_param
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| kkr_core::Error::tool(format!("Request failed: {}", e)))?;

        let status = response.status();
        let body: Value = response
            .json()
            .await
            .map_err(|e| kkr_core::Error::tool(format!("Failed to parse response: {}", e)))?;

        if !status.is_success() {
            return Err(kkr_core::Error::tool(format!("Forex API error: {}", body)));
        }

        Ok(json!({
            "base": body.get("base"),
            "date": body.get("date"),
            "rates": body.get("rates")
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yahoo_quote_metadata() {
        let tool = YahooQuoteTool::new();
        assert_eq!(tool.metadata().category, ToolCategory::Search);
        assert!(tool.metadata().read_only);
    }

    #[test]
    fn test_crypto_quote_metadata() {
        let tool = CryptoQuoteTool::new();
        assert!(tool.metadata().tags.contains(&"crypto".to_string()));
    }

    #[test]
    fn test_forex_metadata() {
        let tool = ForexQuoteTool::new();
        assert!(tool.metadata().tags.contains(&"forex".to_string()));
    }
}
