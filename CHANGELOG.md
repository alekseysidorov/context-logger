# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1] - 2025.05.14

- Fixed `ContextLogger::try_init` method where the wrong object was being passed
  to `log::set_boxed_logger` (issue #3)

## [0.1.0] - 2025.05.11

- Initial release of `context_logger` crate.
