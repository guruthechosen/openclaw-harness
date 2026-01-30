//! Clawdbot auto-patcher module
//!
//! Patches Clawdbot's internal code to wire up `before_tool_call` hooks
//! that aren't connected by default.

pub mod clawdbot;
