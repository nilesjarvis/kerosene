use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::wallet_cluster_state::{
    MAX_WALLET_CLUSTER_MEMBERS, WalletCluster, WalletClusterMember, format_weight_input,
    parse_member_weight, wallet_cluster_window_settings,
};
use iced::{Task, window};

impl TradingTerminal {
    pub(super) fn open_wallet_clusters_window(&mut self) -> Task<Message> {
        if let Some(window_id) = self.wallet_clusters.window_id {
            return window::gain_focus(window_id);
        }

        let settings = wallet_cluster_window_settings(
            &self.wallet_clusters,
            self.custom_window_chrome_active,
            self.window_background_blur_enabled,
        );
        let (window_id, open_task) = window::open(settings);
        self.wallet_clusters.window_id = Some(window_id);
        self.wallet_clusters.open = true;
        self.persist_config();
        Task::batch([
            open_task.map(Message::WindowOpened),
            self.refresh_selected_wallet_cluster(),
        ])
    }

    pub(super) fn create_wallet_cluster(&mut self) -> Task<Message> {
        let name = self.wallet_clusters.new_cluster_name_input.trim();
        let name = if name.is_empty() {
            format!("Cluster {}", self.wallet_clusters.clusters.len() + 1)
        } else {
            name.to_string()
        };
        let id = crate::config::new_secret_id();
        self.wallet_clusters.clusters.push(WalletCluster {
            id: id.clone(),
            name,
            members: Vec::new(),
        });
        self.wallet_clusters.selected_cluster_id = Some(id);
        self.wallet_clusters.new_cluster_name_input.clear();
        self.wallet_clusters.status = None;
        self.persist_config();
        Task::none()
    }

    pub(super) fn select_wallet_cluster(&mut self, cluster_id: String) -> Task<Message> {
        if self
            .wallet_clusters
            .clusters
            .iter()
            .any(|cluster| cluster.id == cluster_id)
        {
            self.wallet_clusters.selected_cluster_id = Some(cluster_id);
            self.wallet_clusters.status = None;
            self.persist_config();
            self.refresh_selected_wallet_cluster()
        } else {
            Task::none()
        }
    }

    pub(super) fn rename_wallet_cluster(
        &mut self,
        cluster_id: String,
        value: String,
    ) -> Task<Message> {
        if let Some(cluster) = self
            .wallet_clusters
            .clusters
            .iter_mut()
            .find(|cluster| cluster.id == cluster_id)
        {
            cluster.name = value;
            self.persist_config();
        }
        Task::none()
    }

    pub(super) fn delete_wallet_cluster(&mut self, cluster_id: String) -> Task<Message> {
        if self.wallet_clusters.has_pending_execution() {
            self.set_wallet_cluster_status(
                "Wait for pending cluster executions to finish before deleting a cluster",
                true,
            );
            return Task::none();
        }
        let before = self.wallet_clusters.clusters.len();
        self.wallet_clusters
            .clusters
            .retain(|cluster| cluster.id != cluster_id);
        if self.wallet_clusters.clusters.len() == before {
            return Task::none();
        }

        if self.wallet_clusters.selected_cluster_id.as_deref() == Some(&cluster_id) {
            self.wallet_clusters.selected_cluster_id = self
                .wallet_clusters
                .clusters
                .first()
                .map(|cluster| cluster.id.clone());
        }
        self.wallet_clusters.member_data.clear();
        self.wallet_clusters.status = None;
        self.persist_config();
        self.refresh_selected_wallet_cluster()
    }

    pub(super) fn add_wallet_cluster_member(&mut self, profile_secret_id: String) -> Task<Message> {
        if self
            .accounts
            .iter()
            .all(|profile| profile.secret_id != profile_secret_id)
        {
            self.set_wallet_cluster_status("Account profile no longer exists", true);
            return Task::none();
        }
        // Watch-only accounts can't sign, so they can never produce a valid
        // cluster leg. Reject them at the boundary (the add-row UI also filters
        // them) instead of only failing later at submission time.
        if self.ghost_account_secret_ids.contains(&profile_secret_id) {
            self.set_wallet_cluster_status(
                "Watch-only accounts cannot be added to a trading cluster",
                true,
            );
            return Task::none();
        }
        // Likewise require a committed agent key (the add-row UI hides keyless
        // profiles); without one the member could never sign a leg.
        if self.accounts.iter().any(|profile| {
            profile.secret_id == profile_secret_id && profile.agent_key.trim().is_empty()
        }) {
            self.set_wallet_cluster_status(
                "Account needs a committed agent key to join a trading cluster",
                true,
            );
            return Task::none();
        }

        let Some(cluster) = self.wallet_clusters.selected_cluster_mut() else {
            self.set_wallet_cluster_status("Create or select a cluster first", true);
            return Task::none();
        };
        if cluster
            .members
            .iter()
            .any(|member| member.profile_secret_id == profile_secret_id)
        {
            self.set_wallet_cluster_status("Account is already in this cluster", true);
            return Task::none();
        }
        if cluster.members.len() >= MAX_WALLET_CLUSTER_MEMBERS {
            self.set_wallet_cluster_status(
                format!("A cluster can include at most {MAX_WALLET_CLUSTER_MEMBERS} wallets"),
                true,
            );
            return Task::none();
        }

        cluster.members.push(WalletClusterMember {
            profile_secret_id: profile_secret_id.clone(),
            weight: crate::config::default_wallet_cluster_member_weight(),
            weight_input: format_weight_input(crate::config::default_wallet_cluster_member_weight()),
        });
        self.persist_config();
        self.refresh_wallet_cluster_member(profile_secret_id)
    }

    pub(super) fn remove_wallet_cluster_member(
        &mut self,
        cluster_id: String,
        profile_secret_id: Option<String>,
    ) -> Task<Message> {
        let Some(profile_secret_id) = profile_secret_id else {
            return Task::none();
        };
        let Some(cluster) = self
            .wallet_clusters
            .clusters
            .iter_mut()
            .find(|cluster| cluster.id == cluster_id)
        else {
            return Task::none();
        };
        cluster
            .members
            .retain(|member| member.profile_secret_id != profile_secret_id);
        self.wallet_clusters.member_data.remove(&profile_secret_id);
        self.persist_config();
        Task::none()
    }

    pub(super) fn change_wallet_cluster_member_weight(
        &mut self,
        cluster_id: String,
        profile_secret_id: Option<String>,
        value: String,
    ) -> Task<Message> {
        let Some(profile_secret_id) = profile_secret_id else {
            return Task::none();
        };
        let Some(cluster) = self
            .wallet_clusters
            .clusters
            .iter_mut()
            .find(|cluster| cluster.id == cluster_id)
        else {
            return Task::none();
        };
        let Some(member) = cluster
            .members
            .iter_mut()
            .find(|member| member.profile_secret_id == profile_secret_id)
        else {
            return Task::none();
        };

        member.weight_input = value.clone();
        if let Some(weight) = parse_member_weight(&value) {
            member.weight = weight;
            self.wallet_clusters.status = None;
            self.persist_config();
        } else {
            self.set_wallet_cluster_status("Member weight must be a non-negative number", true);
        }
        Task::none()
    }
}

#[cfg(test)]
mod tests;
