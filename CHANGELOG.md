# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

- _breaking_ Renamed `LogContext::record` to `LogContext::with_record` to follow
  the standard Rust builder pattern naming convention
- _breaking_ Replaced `LogContext::enter` instance method with the
  `LogScope::enter(context)` static method; `LogScope` is now the explicit guard
  type that keeps the context active and removes it from the stack on drop
- _breaking_ Moved `LogContext::add_record` to `LogScope::add_record`; dynamic
  record insertion is now clearly associated with the active scope rather than
  the context builder
- _breaking_ Renamed `LogContextGuard` to `LogScope` in the public API
- _breaking_ Renamed `ContextValue` to `LogValue` to better reflect its role in
  structured logging

## [0.1.4] - 2026.02.28

- Implemented `Clone` for `ContextValue` and `LogContext`
- Added `From<f32>` conversion for `ContextValue`

## [0.1.3] - 2025.08.29

- Fixed a bug where default records weren't applied without an active context

## [0.1.2] - 2025.08.28

- Added `default_record` method to `ContextLogger` that allows setting default
  records which will be included in all log entries regardless of context

## [0.1.1] - 2025.05.14

- Fixed `ContextLogger::try_init` method where the wrong object was being passed
  to `log::set_boxed_logger` (issue #3)

## [0.1.0] - 2025.05.11

- Initial release of `context_logger` crate.
