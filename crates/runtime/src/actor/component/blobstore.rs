use super::{Ctx, Instance};

use crate::capability::blobstore::blobstore::ContainerName;
use crate::capability::blobstore::container::{Container, StreamObjectNames};
use crate::capability::blobstore::types::{
    ContainerMetadata, Error, ObjectId, ObjectMetadata, ObjectName,
};
use crate::capability::blobstore::{blobstore, container, types};
use crate::capability::Blobstore;
use crate::io::AsyncVec;

use std::sync::Arc;

use anyhow::{bail, ensure, Context as _};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeekExt};
use tracing::instrument;
use wasmtime::component::Resource;
use wasmtime_wasi::preview2::pipe::{AsyncReadStream, AsyncWriteStream};
use wasmtime_wasi::preview2::{HostOutputStream, InputStream};

type Result<T, E = Error> = core::result::Result<T, E>;

impl Instance {
    /// Set [`Blobstore`] handler for this [Instance].
    pub fn blobstore(&mut self, blobstore: Arc<dyn Blobstore + Send + Sync>) -> &mut Self {
        self.handler_mut().replace_blobstore(blobstore);
        self
    }
}

#[async_trait]
impl container::HostContainer for Ctx {
    #[instrument]
    fn drop(&mut self, container: Resource<Container>) -> anyhow::Result<()> {
        self.table
            .delete(container)
            .context("failed to delete container")?;
        Ok(())
    }

    #[instrument]
    async fn name(&mut self, container: Resource<Container>) -> anyhow::Result<Result<String>> {
        let name = self
            .table
            .get(&container)
            .context("failed to get container")?;
        Ok(Ok(name.to_string()))
    }

    #[instrument]
    async fn info(
        &mut self,
        container: Resource<Container>,
    ) -> anyhow::Result<Result<ContainerMetadata>> {
        let name = self
            .table
            .get(&container)
            .context("failed to get container")?;
        match self.handler.container_info(name).await {
            Ok(md) => Ok(Ok(md)),
            Err(err) => Ok(Err(format!("{err:#}"))),
        }
    }

    #[instrument]
    async fn get_data(
        &mut self,
        container: Resource<Container>,
        name: ObjectName,
        start: u64,
        end: u64,
    ) -> anyhow::Result<Result<Resource<types::IncomingValue>>> {
        let container = self
            .table
            .get(&container)
            .context("failed to get container")?;
        match self.handler.get_data(container, name, start..=end).await {
            Ok((stream, size)) => {
                let value = self
                    .table
                    .push((stream, size))
                    .context("failed to push stream and size")?;
                Ok(Ok(value))
            }
            Err(err) => Ok(Err(format!("{err:#}"))),
        }
    }

    #[instrument]
    async fn write_data(
        &mut self,
        container: Resource<Container>,
        name: ObjectName,
        data: Resource<types::OutgoingValue>,
    ) -> anyhow::Result<Result<()>> {
        let mut stream = self
            .table
            .get::<AsyncVec>(&data)
            .context("failed to get outgoing value")?
            .clone();
        stream.rewind().await.context("failed to rewind stream")?;
        let container = self
            .table
            .get(&container)
            .context("failed to get container")?;
        match self
            .handler
            .write_data(container, name, Box::new(stream))
            .await
        {
            Ok(()) => Ok(Ok(())),
            Err(err) => Ok(Err(format!("{err:#}"))),
        }
    }

    #[instrument]
    async fn list_objects(
        &mut self,
        container: Resource<Container>,
    ) -> anyhow::Result<Result<Resource<StreamObjectNames>>> {
        let container = self
            .table
            .get(&container)
            .context("failed to get container")?;
        match self.handler.list_objects(container).await {
            Ok(stream) => {
                let stream = self
                    .table
                    .push(stream)
                    .context("failed to push object name stream")?;
                Ok(Ok(stream))
            }
            Err(err) => Ok(Err(format!("{err:#}"))),
        }
    }

    #[instrument]
    async fn delete_object(
        &mut self,
        container: Resource<Container>,
        name: ObjectName,
    ) -> anyhow::Result<Result<()>> {
        self.delete_objects(container, vec![name]).await
    }

    #[instrument]
    async fn delete_objects(
        &mut self,
        container: Resource<Container>,
        names: Vec<ObjectName>,
    ) -> anyhow::Result<Result<()>> {
        let container = self
            .table
            .get(&container)
            .context("failed to get container")?;
        match self.handler.delete_objects(container, names).await {
            Ok(()) => Ok(Ok(())),
            Err(err) => Ok(Err(format!("{err:#}"))),
        }
    }

    #[instrument]
    async fn has_object(
        &mut self,
        container: Resource<Container>,
        name: ObjectName,
    ) -> anyhow::Result<Result<bool>> {
        let container = self
            .table
            .get(&container)
            .context("failed to get container")?;
        match self.handler.has_object(container, name).await {
            Ok(exists) => Ok(Ok(exists)),
            Err(err) => Ok(Err(format!("{err:#}"))),
        }
    }

    #[instrument]
    async fn object_info(
        &mut self,
        container: Resource<Container>,
        name: ObjectName,
    ) -> anyhow::Result<Result<ObjectMetadata>> {
        let container = self
            .table
            .get(&container)
            .context("failed to get container")?;
        match self.handler.object_info(container, name).await {
            Ok(info) => Ok(Ok(info)),
            Err(err) => Ok(Err(format!("{err:#}"))),
        }
    }

    #[instrument]
    async fn clear(&mut self, container: Resource<Container>) -> anyhow::Result<Result<()>> {
        let container = self
            .table
            .get(&container)
            .context("failed to get container")?;
        match self.handler.clear_container(container).await {
            Ok(()) => Ok(Ok(())),
            Err(err) => Ok(Err(format!("{err:#}"))),
        }
    }
}

#[async_trait]
impl container::HostStreamObjectNames for Ctx {
    #[instrument]
    fn drop(&mut self, names: Resource<StreamObjectNames>) -> anyhow::Result<()> {
        let _: Box<dyn Stream<Item = anyhow::Result<String>> + Sync + Send + Unpin> = self
            .table
            .delete(names)
            .context("failed to delete object name stream")?;
        Ok(())
    }

    #[instrument]
    async fn read_stream_object_names(
        &mut self,
        this: Resource<StreamObjectNames>,
        len: u64,
    ) -> anyhow::Result<Result<(Vec<ObjectName>, bool)>> {
        let stream = self
            .table
            .get_mut(&this)
            .context("failed to get object name stream")?;
        let mut names = Vec::with_capacity(len.try_into().unwrap_or(usize::MAX));
        for _ in 0..len {
            match stream.next().await {
                Some(Ok(name)) => names.push(name),
                Some(Err(err)) => return Ok(Err(format!("{err:#}"))),
                None => return Ok(Ok((names, true))),
            }
        }
        Ok(Ok((names, false)))
    }

    #[instrument]
    async fn skip_stream_object_names(
        &mut self,
        this: Resource<StreamObjectNames>,
        num: u64,
    ) -> anyhow::Result<Result<(u64, bool)>> {
        let stream = self
            .table
            .get_mut(&this)
            .context("failed to get object name stream")?;
        for i in 0..num {
            match stream.next().await {
                Some(Ok(_)) => {}
                Some(Err(err)) => return Ok(Err(format!("{err:#}"))),
                None => return Ok(Ok((i, true))),
            }
        }
        Ok(Ok((num, false)))
    }
}

#[async_trait]
impl types::HostOutgoingValue for Ctx {
    #[instrument]
    fn drop(&mut self, outgoing_value: Resource<types::OutgoingValue>) -> anyhow::Result<()> {
        self.table
            .delete(outgoing_value)
            .context("failed to delete outgoing value")?;
        Ok(())
    }

    #[instrument]
    async fn new_outgoing_value(&mut self) -> anyhow::Result<Resource<types::OutgoingValue>> {
        self.table
            .push(AsyncVec::default())
            .context("failed to push outgoing value")
    }

    #[instrument]
    async fn outgoing_value_write_body(
        &mut self,
        outgoing_value: Resource<types::OutgoingValue>,
    ) -> anyhow::Result<Result<Resource<Box<dyn HostOutputStream>>, ()>> {
        let stream = self
            .table
            .get::<AsyncVec>(&outgoing_value)
            .context("failed to get outgoing value")?
            .clone();
        let stream: Box<dyn HostOutputStream> = Box::new(AsyncWriteStream::new(1 << 16, stream));
        let stream = self
            .table
            .push(stream)
            .context("failed to push output stream")?;
        Ok(Ok(stream))
    }
}

#[async_trait]
impl types::HostIncomingValue for Ctx {
    #[instrument]
    fn drop(&mut self, incoming_value: Resource<types::IncomingValue>) -> anyhow::Result<()> {
        self.table
            .delete(incoming_value)
            .context("failed to delete incoming value")?;
        Ok(())
    }

    #[instrument]
    async fn incoming_value_consume_sync(
        &mut self,
        incoming_value: Resource<types::IncomingValue>,
    ) -> anyhow::Result<Result<types::IncomingValueSyncBody>> {
        let (stream, size) = self
            .table
            .delete(incoming_value)
            .context("failed to delete incoming value")?;
        let mut stream = stream.take(size);
        let size = size.try_into().context("size does not fit in `usize`")?;
        let mut buf = Vec::with_capacity(size);
        match stream.read_to_end(&mut buf).await {
            Ok(n) => {
                ensure!(n == size);
                Ok(Ok(buf))
            }
            Err(err) => Ok(Err(format!("{err:#}"))),
        }
    }

    #[instrument]
    async fn incoming_value_consume_async(
        &mut self,
        incoming_value: Resource<types::IncomingValue>,
    ) -> anyhow::Result<Result<Resource<InputStream>>> {
        let (stream, _) = self
            .table
            .delete(incoming_value)
            .context("failed to delete incoming value")?;
        let stream = self
            .table
            .push(InputStream::Host(Box::new(AsyncReadStream::new(stream))))
            .context("failed to push input stream")?;
        Ok(Ok(stream))
    }

    #[instrument]
    async fn size(
        &mut self,
        incoming_value: Resource<types::IncomingValue>,
    ) -> anyhow::Result<u64> {
        let (_, size): &(Box<dyn AsyncRead + Sync + Send + Unpin>, _) = self
            .table
            .get(&incoming_value)
            .context("failed to get incoming value")?;
        Ok(*size)
    }
}

impl types::Host for Ctx {}

#[async_trait]
impl blobstore::Host for Ctx {
    #[instrument]
    async fn create_container(
        &mut self,
        name: ContainerName,
    ) -> anyhow::Result<Result<Resource<Container>>> {
        match self.handler.create_container(&name).await {
            Ok(()) => {
                let container = self
                    .table
                    .push(Arc::new(name))
                    .context("failed to push container")?;
                Ok(Ok(container))
            }
            Err(err) => Ok(Err(format!("{err:#}"))),
        }
    }

    #[instrument]
    async fn get_container(
        &mut self,
        name: ContainerName,
    ) -> anyhow::Result<Result<Resource<Container>>> {
        match self.handler.container_exists(&name).await {
            Ok(true) => {
                let container = self
                    .table
                    .push(Arc::new(name))
                    .context("failed to push container")?;
                Ok(Ok(container))
            }
            Ok(false) => Ok(Err("container does not exist".into())),
            Err(err) => Ok(Err(format!("{err:#}"))),
        }
    }

    #[instrument]
    async fn delete_container(&mut self, name: ContainerName) -> anyhow::Result<Result<()>> {
        match self.handler.delete_container(&name).await {
            Ok(()) => Ok(Ok(())),
            Err(err) => Ok(Err(format!("{err:#}"))),
        }
    }

    #[instrument]
    async fn container_exists(&mut self, name: ContainerName) -> anyhow::Result<Result<bool>> {
        match self.handler.container_exists(&name).await {
            Ok(exists) => Ok(Ok(exists)),
            Err(err) => Ok(Err(format!("{err:#}"))),
        }
    }

    #[allow(unused)] // TODO: Implement and remove
    #[instrument]
    async fn copy_object(&mut self, src: ObjectId, dest: ObjectId) -> anyhow::Result<Result<()>> {
        bail!("not supported yet")
    }

    #[allow(unused)] // TODO: Implement and remove
    #[instrument]
    async fn move_object(&mut self, src: ObjectId, dest: ObjectId) -> anyhow::Result<Result<()>> {
        bail!("not supported yet")
    }
}

#[async_trait]
impl container::Host for Ctx {}
