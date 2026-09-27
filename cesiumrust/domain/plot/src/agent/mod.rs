//! Agent empowerment layer (plan `Agent 赋能态势标绘`).
//!
//! A small, framework-free surface an external agent (LLM tool-caller, script,
//! service) drives the situational-plot core through — **in-process only**, no
//! MCP / HTTP server and no embedded LLM live here. The design red line: an agent
//! emits only high-level *intent / parameters / document data*; all geometry
//! sampling, projection and rasterisation stay in the deterministic pure core.
//!
//! Four cohesive slices:
//!  * [`action`] — the write path: a JSON [`AgentAction::`](action::AgentAction)
//!    intent protocol compiled + validated into reversible
//!    [`PlotCommand`](crate::ops::PlotCommand)s.
//!  * [`query`] — the read path: a pure [`query`](query::query) filter plus
//!    geometry [`measure_length`](query::measure_length) /
//!    [`measure_area`](query::measure_area) read-outs.
//!  * [`schema`] — the self-describing contract: a hand-written JSON
//!    [`action_json_schema`](schema::action_json_schema) and lossless document
//!    [`export_document_json`](schema::export_document_json) /
//!    [`import_document_json`](schema::import_document_json).
//!  * [`session`] — the aggregation point: [`PlotSession`], the single entry a
//!    host shares (document + history behind one API).

pub mod action;
pub mod query;
pub mod schema;
pub mod session;

pub use action::{apply_action, compile, ActionError, AgentAction, Applied, StylePatch};
pub use query::{measure_area, measure_length, query, ElementSummary, QueryFilter};
pub use schema::{action_json_schema, export_document_json, import_document_json, EXAMPLE_ACTION};
pub use session::PlotSession;
