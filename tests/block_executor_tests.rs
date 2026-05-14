use crucible::expr::{EvalError, Value};
use crucible::parser::parse_block;
use crucible::runtime::{Environment, execute_block};

fn seed_employees_table(env: &Environment) {
    env.database()
        .create_table("employees", ["id", "name", "salary"]);

    env.database()
        .with_table_mut("employees", |table| {
            table
                .insert_row(vec![
                    Value::Number(1.0),
                    Value::Text("Alice".to_string()),
                    Value::Number(100.0),
                ])
                .expect("failed to insert first employee");
            table
                .insert_row(vec![
                    Value::Number(2.0),
                    Value::Text("Bob".to_string()),
                    Value::Number(200.0),
                ])
                .expect("failed to insert second employee");
        })
        .expect("failed to seed employees table");
}

#[test]
fn test_execute_block_returns_15_for_basic_plsql_block() {
    let block = parse_block(
        r#"DECLARE
  x NUMBER := 10;
BEGIN
  x := x + 5;
END;"#,
    )
    .expect("Failed to parse block");

    let env = Environment::default();
    let result = execute_block(&block, &env).expect("Failed to execute block");

    assert_eq!(result, Value::Number(15.0));
    assert_eq!(env.get("x"), None);
}

#[test]
fn test_execute_block_rejects_unknown_declaration_type() {
    let block = parse_block(
        r#"DECLARE
  x GO := 10;
BEGIN
  x := x + 10;
END;"#,
    )
    .expect("Failed to parse block");

    let env = Environment::default();
    let err = execute_block(&block, &env).expect_err("Expected a type error");

    assert!(matches!(err, EvalError::TypeError(message) if message.contains("unknown type")));
    assert_eq!(env.get("x"), None);
}

#[test]
fn test_execute_block_rejects_assignment_type_mismatch() {
    let block = parse_block(
        r#"DECLARE
  x NUMBER := 10;
BEGIN
  x := 'abc';
END;"#,
    )
    .expect("Failed to parse block");

    let env = Environment::default();
    let err = execute_block(&block, &env).expect_err("Expected a type error");

    assert!(matches!(err, EvalError::TypeError(message) if message.contains("expects NUMBER") && message.contains("TEXT")));
    assert_eq!(env.get("x"), None);
}

#[test]
fn test_execute_block_updates_parent_scope_variable() {
    let block = parse_block(
        r#"BEGIN
  x := x + 5;
END;"#,
    )
    .expect("Failed to parse block");

    let env = Environment::default();
    env.set("x", Value::Number(10.0));

    let result = execute_block(&block, &env).expect("Failed to execute block");

    assert_eq!(result, Value::Number(15.0));
    assert_eq!(env.get("x"), Some(Value::Number(15.0)));
}

#[test]
fn test_nested_block_shadowing_keeps_outer_variable() {
    let block = parse_block(
        r#"BEGIN
  DECLARE
    x NUMBER := 10;
  BEGIN
    x := x + 5;
  END;
END;"#,
    )
    .expect("Failed to parse block");

    let env = Environment::default();
    env.set("x", Value::Number(1.0));

    let result = execute_block(&block, &env).expect("Failed to execute block");

    assert_eq!(result, Value::Number(15.0));
    assert_eq!(env.get("x"), Some(Value::Number(1.0)));
}

#[test]
fn test_if_then_branch_executes_first_matching_branch() {
    let block = parse_block(
        r#"DECLARE
  x NUMBER := 10;
BEGIN
  IF x > 5 THEN
    x := x + 5;
  ELSIF x > 20 THEN
    x := x + 100;
  ELSE
    x := 0;
  END IF;
END;"#,
    )
    .expect("Failed to parse block");

    let env = Environment::default();
    let result = execute_block(&block, &env).expect("Failed to execute block");

    assert_eq!(result, Value::Number(15.0));
    assert_eq!(env.get("x"), None);
}

#[test]
fn test_if_elsif_branch_executes_when_first_condition_is_false() {
    let block = parse_block(
        r#"DECLARE
  x NUMBER := 3;
BEGIN
  IF x > 5 THEN
    x := 1;
  ELSIF x > 2 THEN
    x := x + 5;
  ELSE
    x := 0;
  END IF;
END;"#,
    )
    .expect("Failed to parse block");

    let env = Environment::default();
    let result = execute_block(&block, &env).expect("Failed to execute block");

    assert_eq!(result, Value::Number(8.0));
}

#[test]
fn test_if_else_branch_executes_when_all_conditions_are_false() {
    let block = parse_block(
        r#"DECLARE
  x NUMBER := 1;
BEGIN
  IF x > 5 THEN
    x := 1;
  ELSIF x > 2 THEN
    x := 2;
  ELSE
    x := x + 9;
  END IF;
END;"#,
    )
    .expect("Failed to parse block");

    let env = Environment::default();
    let result = execute_block(&block, &env).expect("Failed to execute block");

    assert_eq!(result, Value::Number(10.0));
}

#[test]
fn test_while_loop_increments_until_condition_fails() {
    let block = parse_block(
        r#"DECLARE
  x NUMBER := 0;
BEGIN
  WHILE x < 3 LOOP
    x := x + 1;
  END LOOP;
END;"#,
    )
    .expect("Failed to parse block");

    let env = Environment::default();
    let result = execute_block(&block, &env).expect("Failed to execute block");

    assert_eq!(result, Value::Number(3.0));
    assert_eq!(env.get("x"), None);
}

#[test]
fn test_loop_executes_until_exit_when_condition_is_met() {
    let block = parse_block(
        r#"DECLARE
  x NUMBER := 0;
BEGIN
  LOOP
    x := x + 1;
    EXIT WHEN x = 3;
  END LOOP;
END;"#,
    )
    .expect("Failed to parse block");

    let env = Environment::default();
    let result = execute_block(&block, &env).expect("Failed to execute block");

    assert_eq!(result, Value::Number(3.0));
    assert_eq!(env.get("x"), None);
}

#[test]
fn test_for_loop_accumulates_range_values() {
    let block = parse_block(
        r#"DECLARE
  total NUMBER := 0;
BEGIN
  FOR i IN 1..3 LOOP
    total := total + i;
  END LOOP;
END;"#,
    )
    .expect("Failed to parse block");

    let env = Environment::default();
    env.set("i", Value::Number(99.0));

    let result = execute_block(&block, &env).expect("Failed to execute block");

    assert_eq!(result, Value::Number(6.0));
    assert_eq!(env.get("i"), Some(Value::Number(99.0)));
}

#[test]
fn test_exception_handler_catches_runtime_error_with_others() {
    let block = parse_block(
        r#"DECLARE
  x NUMBER := 10;
BEGIN
  x := x / 0;
EXCEPTION
  WHEN OTHERS THEN
    x := 42;
END;"#,
    )
    .expect("Failed to parse block");

    let env = Environment::default();
    let result = execute_block(&block, &env).expect("Failed to execute block");

    assert_eq!(result, Value::Number(42.0));
    assert_eq!(env.get("x"), None);
}

#[test]
fn test_named_exception_declaration_and_raise() {
    let block = parse_block(
        r#"DECLARE
  e EXCEPTION;
  x NUMBER := 0;
BEGIN
  RAISE e;
EXCEPTION
  WHEN e THEN
    x := 7;
END;"#,
    )
    .expect("Failed to parse block");

    let env = Environment::default();
    let result = execute_block(&block, &env).expect("Failed to execute block");

    assert_eq!(result, Value::Number(7.0));
    assert_eq!(env.get("x"), None);
}

#[test]
fn test_select_into_assigns_selected_row_value() {
    let block = parse_block(
        r#"BEGIN
    SELECT salary INTO selected_salary FROM employees WHERE id = 1;
  END;"#,
    )
    .expect("Failed to parse block");

    let env = Environment::default();
    env.set("selected_salary", Value::Null);
    seed_employees_table(&env);

    let result = execute_block(&block, &env).expect("Failed to execute block");

    assert_eq!(result, Value::Number(1.0));
    assert_eq!(env.get("selected_salary"), Some(Value::Number(100.0)));
}

#[test]
fn test_insert_adds_a_new_row() {
    let block = parse_block(
        r#"BEGIN
    INSERT INTO employees (id, name, salary) VALUES (3, 'Carol', 300);
  END;"#,
    )
    .expect("Failed to parse block");

    let env = Environment::default();
    seed_employees_table(&env);

    let result = execute_block(&block, &env).expect("Failed to execute block");

    assert_eq!(result, Value::Number(1.0));

    let table = env
        .database()
        .table("employees")
        .expect("missing employees table");
    assert_eq!(table.rows().len(), 3);

    let inserted = table
        .rows()
        .iter()
        .find(|row| row.get("id") == Some(&Value::Number(3.0)))
        .expect("missing inserted row");

    assert_eq!(
        inserted.get("name"),
        Some(&Value::Text("Carol".to_string()))
    );
    assert_eq!(inserted.get("salary"), Some(&Value::Number(300.0)));
}

#[test]
fn test_update_changes_matching_rows() {
    let block = parse_block(
        r#"BEGIN
    UPDATE employees SET salary = salary + 25 WHERE id = 1;
  END;"#,
    )
    .expect("Failed to parse block");

    let env = Environment::default();
    seed_employees_table(&env);

    let result = execute_block(&block, &env).expect("Failed to execute block");

    assert_eq!(result, Value::Number(1.0));

    let table = env
        .database()
        .table("employees")
        .expect("missing employees table");
    let updated = table
        .rows()
        .iter()
        .find(|row| row.get("id") == Some(&Value::Number(1.0)))
        .expect("missing updated row");

    assert_eq!(updated.get("salary"), Some(&Value::Number(125.0)));
}

#[test]
fn test_delete_removes_matching_rows() {
    let block = parse_block(
        r#"BEGIN
    DELETE FROM employees WHERE id = 2;
  END;"#,
    )
    .expect("Failed to parse block");

    let env = Environment::default();
    seed_employees_table(&env);

    let result = execute_block(&block, &env).expect("Failed to execute block");

    assert_eq!(result, Value::Number(1.0));

    let table = env
        .database()
        .table("employees")
        .expect("missing employees table");
    assert_eq!(table.rows().len(), 1);
    assert!(
        table
            .rows()
            .iter()
            .all(|row| row.get("id") != Some(&Value::Number(2.0)))
    );
}

#[test]
fn test_create_table_statement_creates_in_memory_table() {
    let block = parse_block(
        r#"BEGIN
    CREATE TABLE employees (id, name, salary);
    INSERT INTO employees (id, name, salary) VALUES (3, 'Carol', 300);
    END;"#,
    )
    .expect("Failed to parse block");

    let env = Environment::default();

    let result = execute_block(&block, &env).expect("Failed to execute block");

    assert_eq!(result, Value::Number(1.0));

    let table = env
        .database()
        .table("employees")
        .expect("missing employees table");
    assert_eq!(table.rows().len(), 1);

    let inserted = table
        .rows()
        .iter()
        .find(|row| row.get("id") == Some(&Value::Number(3.0)))
        .expect("missing inserted row");

    assert_eq!(
        inserted.get("name"),
        Some(&Value::Text("Carol".to_string()))
    );
    assert_eq!(inserted.get("salary"), Some(&Value::Number(300.0)));
}

#[test]
fn test_alter_table_add_constraint_tracks_schema_metadata() {
    let block = parse_block(
        r#"BEGIN
    ALTER TABLE employees ADD CONSTRAINT salary_positive (salary);
    END;"#,
    )
    .expect("Failed to parse block");

    let env = Environment::default();
    seed_employees_table(&env);

    let result = execute_block(&block, &env).expect("Failed to execute block");

    assert_eq!(result, Value::Number(1.0));

    let table = env
        .database()
        .table("employees")
        .expect("missing employees table");
    assert_eq!(table.constraints().len(), 1);
    assert_eq!(table.constraints()[0].name, "salary_positive");
    assert_eq!(table.constraints()[0].columns, vec!["salary".to_string()]);
}

#[test]
fn test_drop_table_statement_removes_table_from_memory() {
    let block = parse_block(
        r#"BEGIN
    DROP TABLE employees;
    END;"#,
    )
    .expect("Failed to parse block");

    let env = Environment::default();
    seed_employees_table(&env);

    let result = execute_block(&block, &env).expect("Failed to execute block");

    assert_eq!(result, Value::Number(1.0));
    assert!(env.database().table("employees").is_none());
}

#[test]
fn test_before_insert_trigger_can_mutate_new_row() {
    let block = parse_block(
        r#"DECLARE
  TRIGGER boost_salary BEFORE INSERT ON employees
  BEGIN
    seen_old_salary := OLD.salary;
    NEW.salary := NEW.salary + 50;
  END;
BEGIN
  INSERT INTO employees (id, name, salary) VALUES (3, 'Carol', 300);
END;"#,
    )
    .expect("Failed to parse block");

    let env = Environment::default();
    env.set("seen_old_salary", Value::Null);
    seed_employees_table(&env);

    let result = execute_block(&block, &env).expect("Failed to execute block");

    assert_eq!(result, Value::Number(1.0));
    assert_eq!(env.get("seen_old_salary"), Some(Value::Null));

    let table = env
        .database()
        .table("employees")
        .expect("missing employees table");
    let inserted = table
        .rows()
        .iter()
        .find(|row| row.get("id") == Some(&Value::Number(3.0)))
        .expect("missing inserted row");

    assert_eq!(inserted.get("salary"), Some(&Value::Number(350.0)));
}

#[test]
fn test_after_update_trigger_reads_old_and_new_values() {
    let block = parse_block(
        r#"DECLARE
  TRIGGER audit_salary AFTER UPDATE ON employees
  BEGIN
    salary_sum := OLD.salary + NEW.salary;
  END;
BEGIN
  UPDATE employees SET salary = 150 WHERE id = 1;
END;"#,
    )
    .expect("Failed to parse block");

    let env = Environment::default();
    env.set("salary_sum", Value::Null);
    seed_employees_table(&env);

    let result = execute_block(&block, &env).expect("Failed to execute block");

    assert_eq!(result, Value::Number(1.0));
    assert_eq!(env.get("salary_sum"), Some(Value::Number(250.0)));

    let table = env
        .database()
        .table("employees")
        .expect("missing employees table");
    let updated = table
        .rows()
        .iter()
        .find(|row| row.get("id") == Some(&Value::Number(1.0)))
        .expect("missing updated row");

    assert_eq!(updated.get("salary"), Some(&Value::Number(150.0)));
}

#[test]
fn test_before_delete_trigger_sees_old_row_and_new_is_null() {
    let block = parse_block(
        r#"DECLARE
  TRIGGER capture_deleted_row BEFORE DELETE ON employees
  BEGIN
    deleted_name := OLD.name;
    deleted_new_salary := NEW.salary;
  END;
BEGIN
  DELETE FROM employees WHERE id = 2;
END;"#,
    )
    .expect("Failed to parse block");

    let env = Environment::default();
    env.set("deleted_name", Value::Null);
    env.set("deleted_new_salary", Value::Null);
    seed_employees_table(&env);

    let result = execute_block(&block, &env).expect("Failed to execute block");

    assert_eq!(result, Value::Number(1.0));
    assert_eq!(
        env.get("deleted_name"),
        Some(Value::Text("Bob".to_string()))
    );
    assert_eq!(env.get("deleted_new_salary"), Some(Value::Null));

    let table = env
        .database()
        .table("employees")
        .expect("missing employees table");
    assert_eq!(table.rows().len(), 1);
    assert!(
        table
            .rows()
            .iter()
            .all(|row| row.get("id") != Some(&Value::Number(2.0)))
    );
}

#[test]
fn test_commit_persists_transactional_insert() {
    let block = parse_block(
        r#"BEGIN
      INSERT INTO employees (id, name, salary) VALUES (3, 'Carol', 300);
      COMMIT;
      END;"#,
    )
    .expect("Failed to parse block");

    let env = Environment::default();
    seed_employees_table(&env);

    let result = execute_block(&block, &env).expect("Failed to execute block");

    assert_eq!(result, Value::Number(1.0));
    assert!(!env.database().is_in_transaction());

    let table = env
        .database()
        .table("employees")
        .expect("missing employees table");
    assert_eq!(table.rows().len(), 3);

    let inserted = table
        .rows()
        .iter()
        .find(|row| row.get("id") == Some(&Value::Number(3.0)))
        .expect("missing inserted row");

    assert_eq!(
        inserted.get("name"),
        Some(&Value::Text("Carol".to_string()))
    );
    assert_eq!(inserted.get("salary"), Some(&Value::Number(300.0)));
}

#[test]
fn test_rollback_discards_transactional_insert_after_selecting_it() {
    let block = parse_block(
        r#"BEGIN
      INSERT INTO employees (id, name, salary) VALUES (3, 'Carol', 300);
      SELECT salary INTO selected_salary FROM employees WHERE id = 3;
      ROLLBACK;
      END;"#,
    )
    .expect("Failed to parse block");

    let env = Environment::default();
    env.set("selected_salary", Value::Null);
    seed_employees_table(&env);

    let result = execute_block(&block, &env).expect("Failed to execute block");

    assert_eq!(result, Value::Number(1.0));
    assert!(!env.database().is_in_transaction());
    assert_eq!(env.get("selected_salary"), Some(Value::Number(300.0)));

    let table = env
        .database()
        .table("employees")
        .expect("missing employees table");
    assert_eq!(table.rows().len(), 2);
    assert!(
        table
            .rows()
            .iter()
            .all(|row| row.get("id") != Some(&Value::Number(3.0)))
    );
}

#[test]
fn test_cursor_open_fetch_and_close_assigns_rows_in_order() {
    let block = parse_block(
        r#"DECLARE
  CURSOR employee_cursor IS SELECT id, salary INTO _ FROM employees;
BEGIN
  OPEN employee_cursor;
  FETCH employee_cursor INTO first_id, first_salary;
  FETCH employee_cursor INTO second_id, second_salary;
  CLOSE employee_cursor;
END;"#,
    )
    .expect("Failed to parse block");

    let env = Environment::default();
    env.set("first_id", Value::Null);
    env.set("first_salary", Value::Null);
    env.set("second_id", Value::Null);
    env.set("second_salary", Value::Null);
    seed_employees_table(&env);

    let result = execute_block(&block, &env).expect("Failed to execute block");

    assert_eq!(result, Value::Number(1.0));
    assert_eq!(env.get("employee_cursor"), None);
    assert_eq!(env.get("first_id"), Some(Value::Number(1.0)));
    assert_eq!(env.get("first_salary"), Some(Value::Number(100.0)));
    assert_eq!(env.get("second_id"), Some(Value::Number(2.0)));
    assert_eq!(env.get("second_salary"), Some(Value::Number(200.0)));
}

#[test]
fn test_cursor_loop_accumulates_selected_values() {
    let block = parse_block(
        r#"DECLARE
  CURSOR employee_cursor IS SELECT salary INTO _ FROM employees;
BEGIN
  FOR salary IN employee_cursor LOOP
    total := total + salary;
  END LOOP;
END;"#,
    )
    .expect("Failed to parse block");

    let env = Environment::default();
    env.set("total", Value::Number(0.0));
    seed_employees_table(&env);

    let result = execute_block(&block, &env).expect("Failed to execute block");

    assert_eq!(result, Value::Number(300.0));
    assert_eq!(env.get("total"), Some(Value::Number(300.0)));
    assert_eq!(env.get("salary"), None);
}
