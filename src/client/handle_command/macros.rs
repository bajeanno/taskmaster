macro_rules! handle_command {
    (
        $args_iter:ident;
        $($cases:tt)*
    ) => {{
        let command_name = $args_iter.next().ok_or(ParseError::EmptyCommand)?;

        handle_command_cases!(
            || Err(ParseError::UnknownCommand(command_name).into()),
            $args_iter,
            command_name,
            /// Prints the available commands
            "help" => {
                print_help_cases!($($cases)*);
            },
            $($cases)*
        )
    }};
}

macro_rules! help_command_name {
    ($command_lit:literal, $command_param:ident) => {
        concat!($command_lit, " <", stringify!($command_param), ">")
    };
    ($command_lit:literal) => {
        $command_lit
    };
}

macro_rules! print_help_cases {
    () => {};

    (
        #[doc = $doc:literal]
        $command_lit:literal => sub_commands! { $($sub_cases:tt)* },
        $($cases:tt)*
    ) => {
        println!("{:<34} {}", $command_lit, $doc);
        print_help_sub_cases!($($sub_cases)*);
        print_help_cases!($($cases)*);
    };

    (
        #[doc = $doc:literal]
        $command_lit:literal $(, $command_param:ident)? => $handler:expr,
        $($cases:tt)*
    ) => {
        println!(
            "{:<34} {}",
            help_command_name!($command_lit $(, $command_param)?),
            $doc
        );
        print_help_cases!($($cases)*);
    };
}

macro_rules! print_help_sub_cases {
    () => {};

    (
        #[doc = $doc:literal]
        $command_lit:literal $(, $command_param:ident)? => $handler:expr,
        $($cases:tt)*
    ) => {
        println!(
            "  {:<32} {}",
            help_command_name!($command_lit $(, $command_param)?),
            $doc
        );
        print_help_sub_cases!($($cases)*);
    };

    (
        #[doc = $doc:literal]
        $command_param:ident => $handler:expr,
        $($cases:tt)*
    ) => {
        println!(
            "  {:<32} {}",
            concat!("<", stringify!($command_param), ">"),
            $doc
        );
        print_help_sub_cases!($($cases)*);
    };
}

macro_rules! handle_command_cases {
    // -- literal => sub_commands! { .. }
    (
        $default:expr,
        $args_iter:ident,
        $command_name:ident,

        #[doc = $doc:literal]
        $command_lit:literal => sub_commands! { $($handler:tt)* },

        $($cases:tt)*
    ) => {
        match $command_name.as_ref() {
            $command_lit => { handle_sub_command!($args_iter, $($handler)*) }
            _ => { handle_command_cases!($default, $args_iter, $command_name, $($cases)*) }
        }
    };

    // -- literal, [ident] => expr
    (
        $default:expr,
        $args_iter:ident,
        $command_name:ident,

        #[doc = $doc:literal]
        $command_lit:literal $(, $command_param:ident)* $(,)? => $handler:expr,

        $($cases:tt)*
    ) => {
        match $command_name.as_ref() {
            $command_lit => {
                $(
                    let $command_param = get_next_arg_without_dash(
                        &mut $args_iter,
                        stringify!($command_param),
                    )?;
                )*
                check_no_additional_args($args_iter)?;
                $handler
            }
            _ => { handle_command_cases!($default, $args_iter, $command_name, $($cases)*) }
        }
    };

    // -- literal => expr
    (
        $default:expr,
        $args_iter:ident,
        $command_name:ident,

        #[doc = $doc:literal]
        $command_lit:literal => $handler:expr,

        $($cases:tt)*
    ) => {
        match $command_name.as_ref() {
            $command_lit => {
                check_no_additional_args($args_iter)?;
                $handler
            }
            _ => { handle_command_cases!($default, $args_iter, $command_name, $($cases)*) }
        }
    };

    // -- [ident] => expr
    (
        $default:expr,
        $args_iter:ident,
        $command_name:ident,

        #[doc = $doc:literal]
        $command_param:ident $(, $additional_command_param:ident)* $(,)? => $handler:expr,
    ) => {{
        let $command_param = check_doesnt_start_with_dash(
            $command_name,
            stringify!($command_param),
        )?;
        $(
            let $additional_command_param = get_next_arg_without_dash(
                &mut $args_iter,
                stringify!($additional_command_param)
            )?;
        )*
        check_no_additional_args($args_iter)?;
        $handler
    }};

    // -- No cases left
    ($default:expr, $args_iter:ident, $command_name:ident, ) => {
        return ($default)()
    };
}

macro_rules! handle_sub_command {
    ($args_iter:ident, $($cases:tt)*) => {{
        let arg = get_next_arg(&mut $args_iter, None)?;

        handle_command_cases!(
            || Err(ParseError::UnexpectedArg(arg).into()),
            $args_iter,
            arg,
            $($cases)*
        )
    }};
}