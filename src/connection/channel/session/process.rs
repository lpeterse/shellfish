use super::exit::Exit;

#[derive(Debug)]
pub struct Stdin;

#[derive(Debug)]
pub struct Stdout;

#[derive(Debug)]
pub struct Stderr;

#[derive(Debug)]
pub struct Process {
    pub stdin: Option<Stdin>,
    pub stdout: Option<Stdout>,
    pub stderr: Option<Stderr>,
}

impl Process {
    async fn kill(&mut self) -> std::io::Result<()> {
        todo!()
    }

    async fn wait(&mut self) -> std::io::Result<Exit> {
        todo!()
    }
}
