// ---------------------------------------------------------------------------
// Assistant State And Tool Models
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AssistantChatMessage {
    pub role: AssistantRole,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssistantRole {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone)]
pub struct AssistantPaneState {
    pub input: String,
    pub history: Vec<AssistantChatMessage>,
    pub models: Vec<String>,
    pub selected_model: Option<String>,
    pub loading: bool,
    pub status_line: Option<String>,
    pub last_error: Option<String>,
    pub use_account_context: bool,
    pub allow_code_execution: bool,
    pub ollama_url: String,
}

impl Default for AssistantPaneState {
    fn default() -> Self {
        Self {
            input: String::new(),
            history: Vec::new(),
            models: Vec::new(),
            selected_model: None,
            loading: false,
            status_line: None,
            last_error: None,
            use_account_context: true,
            allow_code_execution: false,
            ollama_url: "http://127.0.0.1:11434".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AssistantRuntimeContext {
    pub active_symbol: String,
    pub active_timeframe: String,
    pub latest_price: Option<f64>,
    pub account_summary: Option<String>,
    pub connected_address: Option<String>,
    pub hyperdash_api_key: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AssistantTurnInput {
    pub ollama_url: String,
    pub model: String,
    pub user_prompt: String,
    pub context: AssistantRuntimeContext,
    pub use_account_context: bool,
    pub allow_code_execution: bool,
}

#[derive(Debug, Clone)]
pub struct AssistantTurnResult {
    pub trace_lines: Vec<String>,
    pub answer_text: String,
}

#[derive(Debug, Clone)]
pub enum AssistantToolCall {
    DrawdownDca {
        symbol: String,
        interval: String,
        lookback_days: u32,
        tranche_usd: f64,
        drawdown_pct: f64,
    },
    HourlyDca {
        symbol: String,
        lookback_days: u32,
        tranche_usd: f64,
    },
    PriceLookup {
        symbol: String,
        interval: String,
    },
    Candles {
        symbol: String,
        interval: String,
        lookback_days: u32,
    },
    OrderBook {
        symbol: String,
    },
    Symbols,
    AllMids {
        dex: String,
    },
    AccountSnapshot {
        address: String,
    },
    AccountBalance {
        address: String,
    },
    PortfolioHistory {
        address: String,
    },
    IncomeSnapshot {
        address: String,
    },
    LiquidationLevels {
        symbol: String,
        min_price: f64,
        max_price: f64,
        api_key: String,
    },
    LiquidationHeatmap {
        symbol: String,
        min_price: f64,
        max_price: f64,
        start_time: u64,
        end_time: u64,
        api_key: String,
    },
    None,
}

#[derive(Debug, Clone)]
pub struct AssistantPlannedTurn {
    pub ollama_url: String,
    pub model: String,
    pub plan_text: String,
    pub tool_call: AssistantToolCall,
    pub allow_code_execution: bool,
}
