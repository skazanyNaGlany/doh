#[derive(Debug)]
pub struct Context {
    pub current: bool,
    pub name: String,
    pub cluster: String,
    pub auth_info: String,
    pub namespace: String,
}
