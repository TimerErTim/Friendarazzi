use crate::ResultReport;

use async_trait::async_trait;
mod event;
mod use_cases;

pub use crate::application::ExecutionContext;
use error_stack::IntoReport;
pub use event::*;
#[allow(unused_imports)]
pub use use_cases::*;

/// One write operation, performed inside its own unit of work.
///
/// Reads are not use cases: the API facade runs them as one-off `execute_fn` closures.
/// `SessionPrüfen` is the one exception — verifying an access token decides an identity.
#[async_trait]
pub trait UseCase<Output> {
    type Error: IntoReport + Send;

    async fn perform(
        self,
        ctx: ExecutionContext<'_>,
    ) -> ResultReport<Output, <Self::Error as IntoReport>::Context>;
}

/// Background event handler, analogous to [`UseCase`] with `()` output.
#[async_trait]
pub trait EventHandler {
    type Error: IntoReport + Send;

    async fn perform(
        self,
        ctx: ExecutionContext<'_>,
    ) -> ResultReport<(), <Self::Error as IntoReport>::Context>;
}

#[async_trait]
impl<T: EventHandler + Send> UseCase<()> for T {
    type Error = T::Error;

    async fn perform(
        self,
        ctx: ExecutionContext<'_>,
    ) -> ResultReport<(), <Self::Error as IntoReport>::Context> {
        EventHandler::perform(self, ctx).await
    }
}
