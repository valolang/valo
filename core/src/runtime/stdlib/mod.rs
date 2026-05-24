//! Standard-library service boundary.
//!
//! The tree-walking interpreter still dispatches many builtins directly today.
//! This module defines the runtime-owned service layout Valo can grow into:
//! interpreter, VM, embedded host, and package-loaded stdlib code should all be
//! able to share these service domains.

pub mod collections;

#[derive(Debug, Default)]
pub struct RuntimeServices {
    pub console: ConsoleService,
    pub environment: EnvironmentService,
    pub process: ProcessService,
}

#[derive(Debug, Default)]
pub struct ConsoleService;

#[derive(Debug, Default)]
pub struct EnvironmentService;

#[derive(Debug, Default)]
pub struct ProcessService;
