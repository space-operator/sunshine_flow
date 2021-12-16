use std::collections::HashMap;

use crate::{error::Error, Msg};

pub struct Arg<T> {
    pub inner: Option<T>,
    pub name: String,
}

impl<T> Arg<T>
where
    Msg: TryInto<T, Error = Error>,
    T: Clone,
{
    fn remove_from(&self, inputs: &mut HashMap<String, Msg>) -> Result<T, Error> {
        match &self.inner {
            Some(arg) => arg.clone(),
            None => match inputs.remove(&self.name) {
                Some(msg) => msg.try_into(),
                None => Err(Error::ArgumentNotFound(self.name.clone())),
            },
        }
    }
}
