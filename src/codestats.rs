#[derive(Debug, Clone, Default)]
pub struct CodeStats {
    pub files: usize,
    pub blanks: usize,
    pub codes: usize,
}

impl std::ops::AddAssign for CodeStats {
    fn add_assign(&mut self, rhs: Self) {
        self.files += rhs.files;
        self.blanks += rhs.blanks;
        self.codes += rhs.codes;
    }
}
