# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
