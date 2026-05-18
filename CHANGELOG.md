# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `#![prefix(disable)]` file-level attribute to disable automatic 4-byte length
  prefix for `String`, `Bytes` and `Vec<T>` types. When disabled, users manage
  length via `#[auto]` (send direction) and `#[len_ref]` / `#[remaining]`
  (receive direction).
- Validator rule **E016**: reports when `#![prefix(disable)]` is set but a
  receive-struct `String`/`Bytes`/`Vec<T>` field lacks `#[len_ref]` or
  `#[remaining]`.

### Fixed

- `#[auto(field)]` now correctly computes the byte length of `Bytes` fields
  whose default value is a hex integer (e.g. `payload: Bytes = 0xBEAD`).

## [1.0.1] - 2026-05-10

### Changed

- Project version bump.

## [1.0.0] - 2026-05-09

### Added

- Initial release.
- DSL parser for `.pkt` schema files with `chumsky` + `logos`.
- Binary codec (encode/decode) with support for all primitive types,
  `String`/`Bytes`/`Vec<T>` (with automatic 4-byte length prefix),
  fixed arrays `[T; N]`, nested structs, and bit fields.
- Field attributes: `#[auto]`, `#[len_ref]`, `#[remaining]`, `#[if]`,
  `#[bits]`, `#[bit]`, `#[endian]`, `#[checksum]`.
- File-level attributes: `#![version]`, `#![endian]`, `#![import_path]`,
  `#![doc]`.
- Checksum algorithms: CRC-8, CRC-16 (Modbus), CRC-32 (IEEE 802.3),
  XOR, byte-wise sum.
- Schema validation with detailed error codes (E001–E015).
- Import system with cycle detection and path prefix support.
- Benchmark suite (`cargo bench`).
- Fuzz testing.
