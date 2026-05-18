# Changelog

## 2026-05-18

### Added

- `BOOL` as a type keyword, along with boolean runtime support.
- Double-quoted string literal parsing.
- `DATE` literal syntax.
- Procedures and functions with parameters, return types, and cross-block calls.
- Query join modeling for `INNER`, `LEFT`, `RIGHT`, and `OUTER` joins.

### Changed

- `SYSDATE` now initializes values according to the declared target type: `DATE` keeps the date, `TIMESTAMP` keeps the time portion, and `DATETIME` keeps the full date and time.
- The REPL now prints each evaluated statement result inside a `BEGIN` block instead of only the final block value.
- Join resolution now preserves unmatched rows for left, right, and full outer joins.
- Ambiguous unqualified join columns now raise a clear error instead of resolving silently.

### Fixed

- Uninitialized `DATE` variables no longer fall back to `NULL` when initialized with `DATE` literals or `SYSDATE`.
- Join parsing and execution now handle aliases and multiple join kinds consistently.
