use super::{Close, Closed, Context, Frame, Input, Ready, Resize, Result};

/// Consumer callbacks for native window events.
pub trait Handler {
    fn resume(&mut self, _cx: &mut Context<'_>) -> Result<()> {
        Ok(())
    }

    fn suspend(&mut self, _cx: &mut Context<'_>) -> Result<()> {
        Ok(())
    }

    fn ready(&mut self, _win: &mut Ready<'_>) -> Result<()> {
        Ok(())
    }

    fn resize(&mut self, _win: &mut Resize<'_>) -> Result<()> {
        Ok(())
    }

    fn input(&mut self, _input: &mut Input<'_>) -> Result<()> {
        Ok(())
    }

    fn close(&mut self, close: &mut Close<'_>) -> Result<()> {
        close.close();
        Ok(())
    }

    fn closed(&mut self, _closed: &mut Closed<'_>) -> Result<()> {
        Ok(())
    }

    fn draw(&mut self, _frame: &mut Frame<'_>) -> Result<()> {
        Ok(())
    }

    /// Return `true` when the handler needs an idle callback before the loop waits.
    fn wants_idle(&self) -> bool {
        false
    }

    /// Called before the native event loop waits when `wants_idle` is enabled.
    fn idle(&mut self, _cx: &mut Context<'_>) -> Result<()> {
        Ok(())
    }
}
