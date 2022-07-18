pub mod arg_parse {
    pub fn get_path(args: Vec<String>) -> Option<String> {
        if args.len() < 2 {
            return None;
        }

        Some(args[1].clone())
    }
}
