//! Fixed manager entry point; explicit program choices use structured requests.
//! Every exchange goes through the manager's client contract: status reads to
//! the running service, actions and imports through the manager command.
use lvb_model::contract;
use lvb_model::operator_model::{
    Activity, CurrentOverview, CurrentProductDetail, InstallerImportResult, InteractiveOverview,
    Pulse, Receipt, Request, Snapshot,
};
use lvb_view_model::ProductKey;
use std::path::PathBuf;
#[derive(Clone, Debug)]
pub struct SupportReport {
    pub id: String,
    pub sha256: String,
}
pub enum Reply {
    Overview(Box<InteractiveOverview>),
    CurrentOverview(Box<CurrentOverview>),
    Product(Box<CurrentProductDetail>),
    Snapshot(Box<Snapshot>),
    Activity(Activity),
    Pulse(Pulse),
    Receipt(Receipt),
    Imported(InstallerImportResult),
    ReportSaved(std::path::PathBuf),
    Cancelled,
    FrontendSuperseded,
    Error(String),
}
pub enum Query {
    SaveSupportReport(SupportReport),
    PickInstaller,
    Overview,
    CurrentOverview,
    Product(ProductKey),
    Snapshot,
    Activity,
    Pulse,
    Action(Request),
}
impl From<contract::Reply> for Reply {
    fn from(reply: contract::Reply) -> Self {
        match reply {
            contract::Reply::Overview(overview) => Self::Overview(overview),
            contract::Reply::CurrentOverview(overview) => Self::CurrentOverview(overview),
            contract::Reply::Product(detail) => Self::Product(detail),
            contract::Reply::Snapshot(snapshot) => Self::Snapshot(snapshot),
            contract::Reply::Activity(activity) => Self::Activity(activity),
            contract::Reply::Pulse(pulse) => Self::Pulse(pulse),
            contract::Reply::Receipt(receipt) => Self::Receipt(receipt),
            contract::Reply::Imported(imported) => Self::Imported(imported),
            contract::Reply::Delivered => Self::Error("Unexpected manager reply".into()),
        }
    }
}
/// The manager's request for a status read or action leaving this window.
#[cfg(any(target_os = "linux", test))]
fn request(query: Query) -> contract::Request {
    use contract::Read;
    match query {
        Query::Overview => contract::Request::Read(Read::Overview),
        Query::CurrentOverview => contract::Request::Read(Read::CurrentOverview),
        Query::Product(key) => contract::Request::Read(Read::Product(contract::ProductKey {
            environment: key.environment,
            module_sha256: key.module_sha256,
            class_id: key.class_id,
        })),
        Query::Snapshot => contract::Request::Read(Read::Snapshot),
        Query::Activity => contract::Request::Read(Read::Activity),
        Query::Pulse => contract::Request::Read(Read::Pulse),
        Query::Action(request) => contract::Request::Action(request),
        Query::PickInstaller | Query::SaveSupportReport(_) => {
            unreachable!("file choices are handled before exchange")
        }
    }
}
/// One exchange with the manager through `transport`, as this window shows it.
#[cfg(any(target_os = "linux", test))]
fn exchange(
    transport: impl contract::Transport,
    request: &contract::Request,
    input: Option<std::fs::File>,
) -> Result<Reply, String> {
    match contract::Client::new(transport).call(request, input) {
        Ok(reply) => Ok(reply.into()),
        Err(contract::Failure::FrontendSuperseded) => Ok(Reply::FrontendSuperseded),
        Err(contract::Failure::Error(error)) => Err(error),
        Err(contract::Failure::Unavailable) => Err("Bridge service is unavailable".into()),
    }
}
/// Explicit file choices supplied by a frontend. The core has no dialog toolkit.
pub trait FileSelection {
    fn pick_installer(&self) -> Option<PathBuf>;
    fn pick_program(&self) -> Option<PathBuf>;
    fn save_report(&self, report: &SupportReport) -> Option<PathBuf>;
}

#[cfg(all(target_os = "linux", feature = "native-dialogs"))]
pub mod native;

#[cfg(target_os = "linux")]
mod installed;
#[cfg(target_os = "linux")]
pub use installed::Client;

#[cfg(test)]
mod tests {
    use super::*;
    /// Answers every exchange with one fixed result, as a manager would.
    struct Answering(Result<Vec<u8>, contract::Failure>);
    impl contract::Transport for Answering {
        fn exchange(
            &self,
            _: &contract::Request,
            _: Option<std::fs::File>,
        ) -> Result<Vec<u8>, contract::Failure> {
            self.0.clone()
        }
    }
    #[test]
    fn window_queries_reach_the_manager_through_the_contract() {
        let key = ProductKey {
            environment: "fixture-setup".into(),
            module_sha256: "ab".repeat(32),
            class_id: "fixture-class".into(),
        };
        assert_eq!(
            request(Query::Product(key.clone())).command(),
            [
                "operator",
                "product",
                &key.environment,
                &key.module_sha256,
                &key.class_id
            ]
        );
        let pulse = request(Query::Pulse);
        let reply = serde_json::to_vec(&serde_json::json!({"schema":lvb_model::operator_model::OPERATOR_SCHEMA,
            "service_state":"active","dsp":0,"keepers":2,"maintenance":0,
            "pending_transactions":0,"cleanup_unconfirmed":false,"current_generation":"exact-generation",
            "operation":null,"operation_live":false})).unwrap();
        assert!(matches!(
            exchange(Answering(Ok(reply)), &pulse, None),
            Ok(Reply::Pulse(_))
        ));
        assert!(matches!(
            exchange(
                Answering(Err(contract::Failure::FrontendSuperseded)),
                &pulse,
                None
            ),
            Ok(Reply::FrontendSuperseded)
        ));
        let refused = "Error: \"operator_state_changed_refresh\"\n";
        assert_eq!(
            exchange(
                Answering(Err(contract::Failure::Error(refused.into()))),
                &pulse,
                None
            )
            .err()
            .unwrap(),
            refused
        );
        // A reply of another operator model never reaches the window.
        assert!(
            exchange(Answering(Ok(br#"{"schema":28}"#.to_vec())), &pulse, None)
                .err()
                .unwrap()
                .contains(&format!(
                    "operator model {} required",
                    lvb_model::operator_model::OPERATOR_SCHEMA
                ))
        );
    }
}
