//! 领域层：检测项、覆盖度、综合结论。
//!
//! 这一层**不碰网络、不碰终端**——探测把观测值送进来，呈现层把结论拿出去。
//! 判级契约（docs/verdict.md）是 Web 与 CLI 共同的判据，本层是它的 CLI 侧实现。

pub mod checks;
pub mod verdict;
