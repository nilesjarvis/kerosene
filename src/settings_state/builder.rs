use crate::app_state::SensitiveString;
use crate::builder_codes::{BuilderCodeConfig, BuilderFee};

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct BuilderApprovalRequest {
    pub(crate) generation: u64,
    pub(crate) account: String,
    pub(crate) builder: BuilderFee,
    pub(crate) wallet: bool,
}

impl std::fmt::Debug for BuilderApprovalRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BuilderApprovalRequest")
            .field("generation", &self.generation)
            .field("account", &"<redacted>")
            .field("builder", &self.builder)
            .field("wallet", &self.wallet)
            .finish()
    }
}

#[derive(Default)]
pub(crate) struct BuilderCodeSettings {
    pub(crate) config: BuilderCodeConfig,
    pub(crate) address_input: SensitiveString,
    pub(crate) fee_input: String,
    pub(crate) generation: u64,
    pub(crate) pending: Option<BuilderApprovalRequest>,
    pub(crate) approval: Option<(BuilderApprovalRequest, u32)>,
    pub(crate) status: Option<(String, bool)>,
}

impl BuilderCodeSettings {
    pub(crate) fn from_config(config: &BuilderCodeConfig) -> Self {
        let fee_input = if config.fee_tenths_bps == 0 {
            String::new()
        } else {
            format!(
                "{}.{:01}",
                config.fee_tenths_bps / 10,
                config.fee_tenths_bps % 10
            )
        };
        Self {
            config: config.clone(),
            address_input: config.address.clone().into(),
            fee_input,
            ..Self::default()
        }
    }

    pub(crate) fn invalidate(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.pending = None;
        self.approval = None;
        self.status = None;
    }

    pub(crate) fn saved_builder(&self) -> Result<BuilderFee, String> {
        BuilderFee::new(&self.config.address, self.config.fee_tenths_bps)
    }

    pub(crate) fn draft_is_saved(&self) -> bool {
        BuilderFee::from_inputs(&self.address_input, &self.fee_input)
            .ok()
            .zip(self.saved_builder().ok())
            .is_some_and(|(draft, saved)| draft == saved)
    }
}
