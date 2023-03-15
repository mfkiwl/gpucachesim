use std::path::{Path, PathBuf};
use std::process::{Command, Output};

// #[derive(PartialEq, Clone, Debug)]
// pub struct CommandError(Output);

// impl std::error::Error for CommandError {}

// impl std::fmt::Display for CommandError {
//     fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//         write!(f, "command {:?} failed with exit code. {}", self.0.rhs, self.0.lhs,),
//         // match self.0.kind {
//         //     Some(kind) => write!(
//         //         f,
//         //         "adding {} to {} would {} {}",
//         //         self.0.rhs,
//         //         self.0.lhs,
//         //         kind,
//         //         std::any::type_name::<Lhs>(),
//         //     ),
//         //     None => write!(f, "cannot add {} to {}", self.0.rhs, self.0.lhs,),
//         // }
//     }
// }

//     fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
//         self.0
//             .cause
//             .as_deref()
//             .map(arithmetic::error::AsErr::as_err)
//     }
// }

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("missing libtrace.so shared library")]
    MissingSharedLib,

    #[error("command failed {0:?}")]
    Command(Output),

    // #[error(transparent)]
    // Command(#[from] CommandError),
}

pub fn trace<P, A, D>(executable: P, args: A, trace_dir: D) -> Result<(), Error>
where
    P: AsRef<Path>,
    A: IntoIterator,
    <A as IntoIterator>::Item: AsRef<std::ffi::OsStr>,
    D: AsRef<Path>,
{
    let current_exe = PathBuf::from(std::env::current_exe()?);
    let target_dir = current_exe.parent().ok_or(Error::MissingSharedLib)?;
    let tracer_so = target_dir.join("libtrace.so");
    if !tracer_so.is_file() {
        return Err(Error::MissingSharedLib);
    }

    let mut cmd = Command::new(executable.as_ref());
    cmd.args(args);
    cmd.env(
        "TRACES_DIR",
        &trace_dir
            .as_ref()
            .canonicalize()?
            .to_string_lossy()
            .to_string(),
    );
    cmd.env(
        "LD_PRELOAD",
        &tracer_so.canonicalize()?.to_string_lossy().to_string(),
    );

    dbg!(&tracer_so);
    dbg!(&cmd);

    let result = cmd.output()?;
    if !result.status.success() {
        return Err(Error::Command(result).into());
    }
    println!("{}", String::from_utf8_lossy(&result.stdout));
    println!("{}", String::from_utf8_lossy(&result.stderr));
    Ok(())
}

#[cfg(test)]
mod tests {}
