use anyhow::{anyhow, Result};
use windows::core::Error;

pub enum ExpectedError {
    Win32,
    Other,
}

pub fn handle_error<T, F>(
    error_message: &'static str,
    expected_error: ExpectedError,
    func: F,
) -> Result<T>
where
    F: Fn() -> bool,
    T: Default, // Annahme, dass T einen Standardwert hat, den wir im Erfolgsfall zurückgeben können.
{
    if func() {
        match expected_error {
            ExpectedError::Win32 => {
                let error = Error::from_win32();
                let error_message = format!("{}: {}", error_message, error);
                println!("Result: {:?}", error_message);
                Err(anyhow!(error_message))
            }
            ExpectedError::Other => Err(anyhow!(error_message)),
        }
    } else {
        Ok(T::default()) // Gibt den Standardwert von T zurück, wenn kein Fehler auftritt.
    }
}

pub fn throw_error<T>(error_message: &'static str) -> Result<T, anyhow::Error> {
    Err(anyhow::anyhow!(error_message))
}
