use chrono::NaiveDate;
use crucible::ast::{BinaryOp, Expr, JoinKind, RoutineKind, RoutineParameter, Statement, UnaryOp};
use crucible::expr::{Value, eval};
use crucible::parser::{parse_block, parse_expression};
use crucible::runtime::Environment;

#[test]
fn test_parse_arithmetic_with_precedence() {
    // 1 + 2 * 3 should be 1 + (2 * 3) = 7
    let expr = parse_expression("1 + 2 * 3").expect("Failed to parse");

    // Verify structure: Binary { left: 1, op: Add, right: Binary { left: 2, op: Mul, right: 3 } }
    assert_eq!(
        expr,
        Expr::Binary {
            left: Box::new(Expr::Literal(Value::Number(1.0))),
            op: BinaryOp::Add,
            right: Box::new(Expr::Binary {
                left: Box::new(Expr::Literal(Value::Number(2.0))),
                op: BinaryOp::Mul,
                right: Box::new(Expr::Literal(Value::Number(3.0))),
            }),
        }
    );

    // Evaluate
    let env = Environment::default();
    let result = eval(&expr, &env).expect("Failed to evaluate");
    assert_eq!(result, Value::Number(7.0));
}

#[test]
fn test_parse_parenthesized_expression() {
    // (1 + 2) * 3 should be 9
    let expr = parse_expression("(1 + 2) * 3").expect("Failed to parse");

    // Verify structure: Binary { left: Binary { left: 1, op: Add, right: 2 }, op: Mul, right: 3 }
    assert_eq!(
        expr,
        Expr::Binary {
            left: Box::new(Expr::Binary {
                left: Box::new(Expr::Literal(Value::Number(1.0))),
                op: BinaryOp::Add,
                right: Box::new(Expr::Literal(Value::Number(2.0))),
            }),
            op: BinaryOp::Mul,
            right: Box::new(Expr::Literal(Value::Number(3.0))),
        }
    );

    // Evaluate
    let env = Environment::default();
    let result = eval(&expr, &env).expect("Failed to evaluate");
    assert_eq!(result, Value::Number(9.0));
}

#[test]
fn test_parse_variable_arithmetic() {
    // x + y * 2
    let expr = parse_expression("x + y * 2").expect("Failed to parse");

    // Verify structure
    assert_eq!(
        expr,
        Expr::Binary {
            left: Box::new(Expr::Var("x".to_string())),
            op: BinaryOp::Add,
            right: Box::new(Expr::Binary {
                left: Box::new(Expr::Var("y".to_string())),
                op: BinaryOp::Mul,
                right: Box::new(Expr::Literal(Value::Number(2.0))),
            }),
        }
    );

    // Evaluate with environment
    let env = Environment::default();
    env.set("x", Value::Number(10.0));
    env.set("y", Value::Number(5.0));
    let result = eval(&expr, &env).expect("Failed to evaluate");
    assert_eq!(result, Value::Number(20.0)); // 10 + (5 * 2) = 20
}

#[test]
fn test_parse_double_quoted_string_literal() {
    let expr = parse_expression("\"Hello, World!\"").expect("Failed to parse");

    assert_eq!(
        expr,
        Expr::Literal(Value::Text("Hello, World!".to_string()))
    );
}

#[test]
fn test_parse_date_literal() {
    let expr = parse_expression(r#"DATE "2026-05-14""#).expect("Failed to parse");

    assert_eq!(
        expr,
        Expr::Literal(Value::Date(
            NaiveDate::from_ymd_opt(2026, 5, 14).expect("invalid date")
        ))
    );
}

#[test]
fn test_parse_sysdate_literal() {
    let expr = parse_expression("SYSDATE").expect("Failed to parse");

    // SYSDATE should parse as Expr::Sysdate
    assert_eq!(expr, Expr::Sysdate);
}

#[test]
fn test_parse_logical_not() {
    // NOT FALSE
    let expr = parse_expression("NOT FALSE").expect("Failed to parse");

    // Verify structure
    assert_eq!(
        expr,
        Expr::Unary {
            op: UnaryOp::Not,
            expr: Box::new(Expr::Literal(Value::Bool(false))),
        }
    );

    // Evaluate
    let env = Environment::default();
    let result = eval(&expr, &env).expect("Failed to evaluate");
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_parse_comparison_with_and() {
    // x > 5 AND y < 10
    let expr = parse_expression("x > 5 AND y < 10").expect("Failed to parse");

    // Verify structure: Binary { left: Binary { left: x, op: Gt, right: 5 }, op: And, right: Binary { left: y, op: Lt, right: 10 } }
    assert_eq!(
        expr,
        Expr::Binary {
            left: Box::new(Expr::Binary {
                left: Box::new(Expr::Var("x".to_string())),
                op: BinaryOp::Gt,
                right: Box::new(Expr::Literal(Value::Number(5.0))),
            }),
            op: BinaryOp::And,
            right: Box::new(Expr::Binary {
                left: Box::new(Expr::Var("y".to_string())),
                op: BinaryOp::Lt,
                right: Box::new(Expr::Literal(Value::Number(10.0))),
            }),
        }
    );

    // Evaluate with environment
    let env = Environment::default();
    env.set("x", Value::Number(10.0));
    env.set("y", Value::Number(5.0));
    let result = eval(&expr, &env).expect("Failed to evaluate");
    assert_eq!(result, Value::Bool(true)); // (10 > 5) AND (5 < 10) = true AND true = true
}

#[test]
fn test_parse_comparison_with_and_false() {
    // x > 5 AND y < 10 with x=3 should be false
    let expr = parse_expression("x > 5 AND y < 10").expect("Failed to parse");

    let env = Environment::default();
    env.set("x", Value::Number(3.0));
    env.set("y", Value::Number(5.0));
    let result = eval(&expr, &env).expect("Failed to evaluate");
    assert_eq!(result, Value::Bool(false)); // (3 > 5) AND (5 < 10) = false AND true = false
}

// Block parsing tests
#[test]
fn test_parse_simple_block_without_declare() {
    // BEGIN x := 5; END;
    let block = parse_block("BEGIN x := 5; END;").expect("Failed to parse block");

    // Verify structure
    assert_eq!(block.declarations.len(), 0);
    assert_eq!(block.statements.len(), 1);

    match &block.statements[0] {
        Statement::Assignment { name, value } => {
            assert_eq!(name, "x");
            assert_eq!(*value, Expr::Literal(Value::Number(5.0)));
        }
        _ => panic!("Expected assignment statement"),
    }
}

#[test]
fn test_parse_block_with_declare() {
    // DECLARE x NUMBER := 10; BEGIN x := x + 5; END;
    let block = parse_block("DECLARE x NUMBER := 10; BEGIN x := x + 5; END;")
        .expect("Failed to parse block");

    // Verify declarations
    assert_eq!(block.declarations.len(), 1);
    assert_eq!(block.declarations[0].name, "x");
    assert_eq!(block.declarations[0].type_name, "NUMBER");
    assert_eq!(
        block.declarations[0].init_value,
        Some(Expr::Literal(Value::Number(10.0)))
    );

    // Verify statements
    assert_eq!(block.statements.len(), 1);
    match &block.statements[0] {
        Statement::Assignment { name, value } => {
            assert_eq!(name, "x");
            // x + 5 expression
            assert_eq!(
                *value,
                Expr::Binary {
                    left: Box::new(Expr::Var("x".to_string())),
                    op: BinaryOp::Add,
                    right: Box::new(Expr::Literal(Value::Number(5.0))),
                }
            );
        }
        _ => panic!("Expected assignment statement"),
    }
}

#[test]
fn test_parse_block_with_double_quoted_string_literal() {
    let block = parse_block(
        r#"DECLARE
  x TEXT := "Hello, World!";
BEGIN
  x := x + " this is a test";
END;"#,
    )
    .expect("Failed to parse block");

    assert_eq!(block.declarations.len(), 1);
    assert_eq!(block.declarations[0].name, "x");
    assert_eq!(block.declarations[0].type_name, "TEXT");
    assert_eq!(
        block.declarations[0].init_value,
        Some(Expr::Literal(Value::Text("Hello, World!".to_string())))
    );

    assert_eq!(block.statements.len(), 1);
    match &block.statements[0] {
        Statement::Assignment { name, value } => {
            assert_eq!(name, "x");
            assert_eq!(
                *value,
                Expr::Binary {
                    left: Box::new(Expr::Var("x".to_string())),
                    op: BinaryOp::Add,
                    right: Box::new(Expr::Literal(Value::Text(" this is a test".to_string()))),
                }
            );
        }
        _ => panic!("Expected assignment statement"),
    }
}

#[test]
fn test_parse_block_with_date_literal_initializer() {
    let block = parse_block(
        r#"DECLARE
  x DATE := DATE "2026-05-14";
BEGIN
  x;
END;"#,
    )
    .expect("Failed to parse block");

    assert_eq!(block.declarations.len(), 1);
    assert_eq!(block.declarations[0].name, "x");
    assert_eq!(block.declarations[0].type_name, "DATE");
    assert_eq!(
        block.declarations[0].init_value,
        Some(Expr::Literal(Value::Date(
            NaiveDate::from_ymd_opt(2026, 5, 14).expect("invalid date")
        )))
    );
}

#[test]
fn test_parse_block_with_sysdate_initializer() {
    let block = parse_block(
        r#"DECLARE
  x DATE := SYSDATE;
BEGIN
  x;
END;"#,
    )
    .expect("Failed to parse block");

    assert_eq!(block.declarations.len(), 1);
    assert_eq!(block.declarations[0].name, "x");
    assert_eq!(block.declarations[0].type_name, "DATE");

    // Verify the init_value is Expr::Sysdate
    match &block.declarations[0].init_value {
        Some(Expr::Sysdate) => (),
        _ => panic!("Expected SYSDATE initializer to be Expr::Sysdate"),
    }
}

#[test]
fn test_parse_function_declaration_and_return_statement() {
    let block = parse_block(
        r#"DECLARE
  FUNCTION add(a NUMBER, b NUMBER) RETURN NUMBER IS
  BEGIN
    RETURN a + b;
  END;
BEGIN
  add(1, 2);
END;"#,
    )
    .expect("Failed to parse block");

    assert_eq!(block.declarations.len(), 1);
    assert_eq!(block.declarations[0].type_name, "FUNCTION");
    let routine = block.declarations[0]
        .routine
        .as_ref()
        .expect("Expected routine declaration");
    assert_eq!(routine.kind, RoutineKind::Function);
    assert_eq!(routine.name, "add");
    assert_eq!(
        routine.parameters,
        vec![
            RoutineParameter {
                name: "a".to_string(),
                type_name: "NUMBER".to_string(),
            },
            RoutineParameter {
                name: "b".to_string(),
                type_name: "NUMBER".to_string(),
            },
        ]
    );
    assert_eq!(routine.return_type.as_deref(), Some("NUMBER"));
    assert_eq!(routine.body.statements.len(), 1);
    assert_eq!(
        routine.body.statements[0],
        Statement::Return(Some(Expr::Binary {
            left: Box::new(Expr::Var("a".to_string())),
            op: BinaryOp::Add,
            right: Box::new(Expr::Var("b".to_string())),
        }))
    );
}

#[test]
fn test_parse_procedure_declaration_uses_procedure_type_name() {
    let block = parse_block(
        r#"DECLARE
  PROCEDURE bump_total(value NUMBER) IS
  BEGIN
    total := value;
  END;
BEGIN
END;"#,
    )
    .expect("Failed to parse block");

    assert_eq!(block.declarations.len(), 1);
    assert_eq!(block.declarations[0].type_name, "PROCEDURE");
    let routine = block.declarations[0]
        .routine
        .as_ref()
        .expect("Expected routine declaration");
    assert_eq!(routine.kind, RoutineKind::Procedure);
}

#[test]
fn test_parse_procedure_call_statement() {
    let block = parse_block(
        r#"BEGIN
  bump_total(42);
END;"#,
    )
    .expect("Failed to parse block");

    assert_eq!(block.declarations.len(), 0);
    assert_eq!(block.statements.len(), 1);
    assert_eq!(
        block.statements[0],
        Statement::Call {
            name: "bump_total".to_string(),
            args: vec![Expr::Literal(Value::Number(42.0))],
        }
    );
}

#[test]
fn test_parse_select_into_with_join_and_aliases() {
    let block = parse_block(
        r#"DECLARE
  result TEXT;
BEGIN
  SELECT e.name INTO result
  FROM employees e
  JOIN departments d ON e.id = d.employee_id
  WHERE d.department = "Sales";
END;"#,
    )
    .expect("Failed to parse block");

    assert_eq!(block.statements.len(), 1);

    let Statement::SelectInto(stmt) = &block.statements[0] else {
        panic!("Expected SELECT INTO statement");
    };

    assert_eq!(stmt.targets, vec!["result".to_string()]);
    assert_eq!(stmt.query.select_list.len(), 1);
    assert_eq!(stmt.query.from.table, "employees");
    assert_eq!(stmt.query.from.alias.as_deref(), Some("e"));
    assert_eq!(stmt.query.joins.len(), 1);
    assert_eq!(stmt.query.joins[0].kind, JoinKind::Inner);
    assert_eq!(stmt.query.joins[0].source.table, "departments");
    assert_eq!(stmt.query.joins[0].source.alias.as_deref(), Some("d"));
    assert!(stmt.query.where_clause.is_some());
}

#[test]
fn test_parse_select_into_with_explicit_join_kinds() {
    let block = parse_block(
        r#"DECLARE
  inner_name TEXT;
  left_name TEXT;
  right_name TEXT;
  outer_name TEXT;
BEGIN
  SELECT e.name INTO inner_name
  FROM employees e
  INNER JOIN departments d ON e.id = d.employee_id;

  SELECT e.name INTO left_name
  FROM employees e
  LEFT OUTER JOIN departments d ON e.id = d.employee_id;

  SELECT e.name INTO right_name
  FROM employees e
  RIGHT JOIN departments d ON e.id = d.employee_id;

  SELECT e.name INTO outer_name
  FROM employees e
  OUTER JOIN departments d ON e.id = d.employee_id;
END;"#,
    )
    .expect("Failed to parse block");

    assert_eq!(block.statements.len(), 4);

    let Statement::SelectInto(inner_stmt) = &block.statements[0] else {
        panic!("Expected first statement to be SELECT INTO");
    };
    assert_eq!(inner_stmt.query.joins[0].kind, JoinKind::Inner);

    let Statement::SelectInto(left_stmt) = &block.statements[1] else {
        panic!("Expected second statement to be SELECT INTO");
    };
    assert_eq!(left_stmt.query.joins[0].kind, JoinKind::Left);

    let Statement::SelectInto(right_stmt) = &block.statements[2] else {
        panic!("Expected third statement to be SELECT INTO");
    };
    assert_eq!(right_stmt.query.joins[0].kind, JoinKind::Right);

    let Statement::SelectInto(outer_stmt) = &block.statements[3] else {
        panic!("Expected fourth statement to be SELECT INTO");
    };
    assert_eq!(outer_stmt.query.joins[0].kind, JoinKind::Outer);
}

#[test]
fn test_parse_function_call_expression() {
    let expr = parse_expression("add(1, 2)").expect("Failed to parse");

    assert_eq!(
        expr,
        Expr::Call {
            name: "add".to_string(),
            args: vec![
                Expr::Literal(Value::Number(1.0)),
                Expr::Literal(Value::Number(2.0))
            ],
        }
    );
}

#[test]
fn test_parse_block_with_multiple_declarations() {
    // DECLARE x NUMBER; y NUMBER := 5; BEGIN x := y; END;
    let block = parse_block("DECLARE x NUMBER; y NUMBER := 5; BEGIN x := y; END;")
        .expect("Failed to parse block");

    // Verify declarations
    assert_eq!(block.declarations.len(), 2);

    assert_eq!(block.declarations[0].name, "x");
    assert_eq!(block.declarations[0].type_name, "NUMBER");
    assert_eq!(block.declarations[0].init_value, None);

    assert_eq!(block.declarations[1].name, "y");
    assert_eq!(block.declarations[1].type_name, "NUMBER");
    assert_eq!(
        block.declarations[1].init_value,
        Some(Expr::Literal(Value::Number(5.0)))
    );

    // Verify statements
    assert_eq!(block.statements.len(), 1);
}

#[test]
fn test_parse_block_with_multiple_statements() {
    // BEGIN x := 5; y := 10; END;
    let block = parse_block("BEGIN x := 5; y := 10; END;").expect("Failed to parse block");

    assert_eq!(block.declarations.len(), 0);
    assert_eq!(block.statements.len(), 2);

    match &block.statements[0] {
        Statement::Assignment { name, value } => {
            assert_eq!(name, "x");
            assert_eq!(*value, Expr::Literal(Value::Number(5.0)));
        }
        _ => panic!("Expected assignment statement"),
    }

    match &block.statements[1] {
        Statement::Assignment { name, value } => {
            assert_eq!(name, "y");
            assert_eq!(*value, Expr::Literal(Value::Number(10.0)));
        }
        _ => panic!("Expected assignment statement"),
    }
}

#[test]
fn test_parse_block_with_expression_statement() {
    // BEGIN 1 + 2; END;
    let block = parse_block("BEGIN 1 + 2; END;").expect("Failed to parse block");

    assert_eq!(block.declarations.len(), 0);
    assert_eq!(block.statements.len(), 1);

    match &block.statements[0] {
        Statement::Expression(expr) => {
            assert_eq!(
                *expr,
                Expr::Binary {
                    left: Box::new(Expr::Literal(Value::Number(1.0))),
                    op: BinaryOp::Add,
                    right: Box::new(Expr::Literal(Value::Number(2.0))),
                }
            );
        }
        _ => panic!("Expected expression statement"),
    }
}
