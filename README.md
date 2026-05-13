# Crucible

Crucible is a Rust interpreter for a PL/SQL-like block language. The goal is to stay close to the original PL/SQL model where practical while keeping the implementation small, testable, and easy to extend. It parses anonymous blocks, executes them against an in-memory database, and supports a growing subset of procedural SQL features including DML, transactions, cursors, and row-level triggers.

## Current Status

### Working Now

- [x] Anonymous PL/SQL-style blocks with optional `DECLARE`, required `BEGIN` & `END`, and optional `EXCEPTION` sections.
- [x] Variable declarations, exception declarations, cursor declarations, and trigger declarations.
- [x] Expressions with arithmetic, comparison, logical operators, boolean literals, strings, numbers, and dotted field access.
- [x] Control flow with `IF` / `ELSIF` / `ELSE`, `WHILE`, `LOOP`, numeric `FOR`, cursor `FOR`, and `EXIT WHEN`.
- [x] Exception handling with named handlers and `RAISE`.
- [x] In-memory DML simulation for `SELECT INTO`, `INSERT`, `UPDATE`, and `DELETE`.
- [x] In-memory table creation with `CREATE TABLE`, stored in the current database snapshot.
- [x] In-memory schema changes with `ALTER TABLE ... ADD CONSTRAINT` and `DROP TABLE`.
- [x] Transaction support with `COMMIT` and `ROLLBACK`.
- [x] Cursor support with `CURSOR`, `OPEN`, `FETCH INTO`, `CLOSE`, and cursor loops.
- [x] Row triggers with `BEFORE` and `AFTER` timing for `INSERT`, `UPDATE`, and `DELETE`.
- [x] `OLD` and `NEW` pseudo-records inside trigger bodies, including the ability to mutate `NEW` in `BEFORE` triggers.
- [x] Nested blocks and scoped execution.

### Roadmap

- [ ] Procedures and functions with parameters and return values.
- [ ] Package-level state and package bodies.
- [ ] Savepoints and more complete transactional control.
- [ ] Statement-level triggers and richer trigger conditions.
- [ ] Broader SQL support, including more query forms and joins.
- [ ] Reading, parsing, and validating SQL files in later versions.
- [ ] File-backed schema and table persistence.
- [ ] Stronger type checking and explicit conversions closer to PL/SQL semantics.

## Example

```sql
DECLARE
  TRIGGER boost_salary BEFORE INSERT ON employees
  BEGIN
    NEW.salary := NEW.salary + 50;
  END;
BEGIN
  INSERT INTO employees (id, name, salary) VALUES (3, 'Carol', 300);
END;
```

In this example, the trigger adjusts the inserted row before it is committed to the table snapshot.

## Running

Start the interactive prompt with:

```bash
cargo run
```

The CLI will prompt for a PL/SQL block, then parse and execute it against a fresh in-memory environment.

Run the test suite with:

```bash
cargo test
```

## Project Layout

- `src/parser` parses blocks, expressions, DML, cursors, transactions, and triggers.
- `src/runtime` executes blocks, manages variables, handles DML, cursors, transactions, and triggers.
- `src/db` contains the in-memory database and table model.
- `src/expr` defines runtime values and expression evaluation.
- `tests` contains integration tests that exercise the interpreter end to end.

This project is intentionally incremental, but the long-term direction is to remain recognizable to PL/SQL users instead of becoming a generic scripting language with SQL-like syntax.
