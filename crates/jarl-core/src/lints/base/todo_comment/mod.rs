pub(crate) mod todo_comment;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "todo_comment", None)
    }

    #[test]
    fn test_no_lint_todo_comment() {
        for code in [
            "",
            "x <- 1",
            "# ordinary comment",
            "# TO DO: later",
            "# TODOLOGY",
            "# TODOa123",
            "# fixmeLater",
            "# TODO中文",
            "# FIXMEé",
            "# TODOλ",
            "# FIXMEЖ",
            "# remember TODO: later",
            "# text # TODO: later",
            "#' @description TODO: later",
            "# @TODO",
            "# | TODO",
            r##"x <- "# TODO""##,
            r##"x <- r"(# FIXME)""##,
            "x <- \"line one\n# TODO\nline three\"",
            "# jarl-ignore-file todo_comment: tracked elsewhere\n# TODO: later",
            "# jarl-ignore-start todo_comment: tracked elsewhere\n# FIXME\n# jarl-ignore-end todo_comment",
        ] {
            expect_no_lint(code, "todo_comment", None);
        }
    }

    #[test]
    fn test_lint_todo_comment() {
        assert_snapshot!(
            snapshot_lint("# TODO"),
            @"
        warning: todo_comment
         --> <test>:1:1
          |
        1 | # TODO
          | ------ Remove TODO comments.
          |
        Found 1 error.
        "
        );

        assert_snapshot!(
            snapshot_lint("# FIXME"),
            @"
        warning: todo_comment
         --> <test>:1:1
          |
        1 | # FIXME
          | ------- Remove TODO comments.
          |
        Found 1 error.
        "
        );

        assert_snapshot!(
            snapshot_lint("# tOdO later"),
            @"
        warning: todo_comment
         --> <test>:1:1
          |
        1 | # tOdO later
          | ------------ Remove TODO comments.
          |
        Found 1 error.
        "
        );

        assert_snapshot!(
            snapshot_lint("# fIxMe\tlater"),
            @"
        warning: todo_comment
         --> <test>:1:1
          |
        1 | # fIxMe    later
          | ---------------- Remove TODO comments.
          |
        Found 1 error.
        "
        );

        assert_snapshot!(
            snapshot_lint("#TODO123"),
            @"
        warning: todo_comment
         --> <test>:1:1
          |
        1 | #TODO123
          | -------- Remove TODO comments.
          |
        Found 1 error.
        "
        );

        assert_snapshot!(
            snapshot_lint("# TODO123abc"),
            @"
        warning: todo_comment
         --> <test>:1:1
          |
        1 | # TODO123abc
          | ------------ Remove TODO comments.
          |
        Found 1 error.
        "
        );

        assert_snapshot!(
            snapshot_lint("# FIXME９"),
            @"
        warning: todo_comment
         --> <test>:1:1
          |
        1 | # FIXME９
          | --------- Remove TODO comments.
          |
        Found 1 error.
        "
        );

        assert_snapshot!(
            snapshot_lint("# TODO: later"),
            @"
        warning: todo_comment
         --> <test>:1:1
          |
        1 | # TODO: later
          | ------------- Remove TODO comments.
          |
        Found 1 error.
        "
        );

        assert_snapshot!(
            snapshot_lint("# FIXME(issue): later"),
            @"
        warning: todo_comment
         --> <test>:1:1
          |
        1 | # FIXME(issue): later
          | --------------------- Remove TODO comments.
          |
        Found 1 error.
        "
        );

        assert_snapshot!(
            snapshot_lint("# TODO_task"),
            @"
        warning: todo_comment
         --> <test>:1:1
          |
        1 | # TODO_task
          | ----------- Remove TODO comments.
          |
        Found 1 error.
        "
        );

        assert_snapshot!(
            snapshot_lint("# FIXME.md"),
            @"
        warning: todo_comment
         --> <test>:1:1
          |
        1 | # FIXME.md
          | ---------- Remove TODO comments.
          |
        Found 1 error.
        "
        );

        assert_snapshot!(
            snapshot_lint("# TODO🛠"),
            @"
        warning: todo_comment
         --> <test>:1:1
          |
        1 | # TODO🛠
          | ------- Remove TODO comments.
          |
        Found 1 error.
        "
        );

        assert_snapshot!(
            snapshot_lint("### TODO: later"),
            @"
        warning: todo_comment
         --> <test>:1:1
          |
        1 | ### TODO: later
          | --------------- Remove TODO comments.
          |
        Found 1 error.
        "
        );

        assert_snapshot!(
            snapshot_lint("#\tTODO\u{a0}later"),
            @"
        warning: todo_comment
         --> <test>:1:1
          |
        1 | #    TODO later
          | --------------- Remove TODO comments.
          |
        Found 1 error.
        "
        );

        assert_snapshot!(
            snapshot_lint("#' FIXME: document this"),
            @"
        warning: todo_comment
         --> <test>:1:1
          |
        1 | #' FIXME: document this
          | ----------------------- Remove TODO comments.
          |
        Found 1 error.
        "
        );

        assert_snapshot!(
            snapshot_lint("x <- 1 # TODO: later"),
            @"
        warning: todo_comment
         --> <test>:1:8
          |
        1 | x <- 1 # TODO: later
          |        ------------- Remove TODO comments.
          |
        Found 1 error.
        "
        );

        assert_snapshot!(
            snapshot_lint("function() {\n  # FIXME: later\n  x <- 1\n}"),
            @"
        warning: todo_comment
         --> <test>:2:3
          |
        2 |   # FIXME: later
          |   -------------- Remove TODO comments.
          |
        Found 1 error.
        "
        );

        assert_snapshot!(
            snapshot_lint("# TODO and FIXME: one diagnostic"),
            @"
        warning: todo_comment
         --> <test>:1:1
          |
        1 | # TODO and FIXME: one diagnostic
          | -------------------------------- Remove TODO comments.
          |
        Found 1 error.
        "
        );
    }
}
