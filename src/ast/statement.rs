use super::{
    AlterTableStatement, Block, CreateTableStatement, DeleteStatement, DropTableStatement, Expr,
    IfBranch, InsertStatement, SelectIntoTarget, UpdateStatement,
};

#[derive(Clone, Debug, PartialEq)]
pub enum Statement {
    Block(Box<Block>),
    If {
        condition: Expr,
        then_branch: Vec<Statement>,
        elsif_branches: Vec<IfBranch>,
        else_branch: Option<Vec<Statement>>,
    },
    While {
        condition: Expr,
        body: Vec<Statement>,
    },
    Loop {
        body: Vec<Statement>,
    },
    For {
        variable: String,
        start: Expr,
        end: Expr,
        body: Vec<Statement>,
    },
    Exit {
        condition: Option<Expr>,
    },
    Raise {
        name: String,
    },
    Commit,
    Rollback,
    OpenCursor {
        name: String,
    },
    FetchCursor {
        name: String,
        targets: Vec<String>,
    },
    CloseCursor {
        name: String,
    },
    CursorFor {
        targets: Vec<String>,
        cursor: String,
        body: Vec<Statement>,
    },
    SelectInto(SelectIntoTarget),
    Insert(InsertStatement),
    Update(UpdateStatement),
    Delete(DeleteStatement),
    CreateTable(CreateTableStatement),
    AlterTable(AlterTableStatement),
    DropTable(DropTableStatement),
    Call {
        name: String,
        args: Vec<Expr>,
    },
    Return(Option<Expr>),
    Expression(Expr),
    Assignment {
        name: String,
        value: Expr,
    },
}
