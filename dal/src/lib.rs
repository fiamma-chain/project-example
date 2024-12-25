use connection::holder::ConnectionHolder;
use sqlx::{pool::PoolConnection, Connection, PgConnection, Postgres, Transaction};

pub mod connection;

pub fn get_master_database_url() -> String {
    if std::env::var("FIAMME_BRIDGE_IN_DOCKER")
        .unwrap_or_else(|_| "false".to_string())
        .parse()
        .unwrap_or(false)
    {
        "postgres://admin:admin123@host.docker.internal:7432/bitvm-bridge".into()
    } else {
        "postgres://admin:admin123@localhost:7432/bitvm-bridge".into()
    }
}

#[derive(Debug)]
pub struct StorageProcessor<'a> {
    conn: ConnectionHolder<'a>,
    in_transaction: bool,
}

impl<'a> StorageProcessor<'a> {
    pub async fn establish_connection(connection_to_master: bool) -> StorageProcessor<'static> {
        let db_url = if connection_to_master {
            get_master_database_url()
        } else {
            panic!("not support other database")
        };
        let connection = PgConnection::connect(&db_url).await.unwrap();
        StorageProcessor {
            conn: ConnectionHolder::Direct(connection),
            in_transaction: false,
        }
    }

    pub async fn start_transaction<'c: 'b, 'b>(&'c mut self) -> StorageProcessor<'b> {
        let transaction = self.conn().begin().await.unwrap();

        let mut processor = StorageProcessor::from_transaction(transaction);
        processor.in_transaction = true;

        processor
    }

    pub fn from_transaction(conn: Transaction<'a, Postgres>) -> Self {
        Self {
            conn: ConnectionHolder::Transaction(conn),
            in_transaction: true,
        }
    }

    pub fn from_pool(conn: PoolConnection<Postgres>) -> Self {
        Self {
            conn: ConnectionHolder::Pooled(conn),
            in_transaction: false,
        }
    }

    pub fn conn(&mut self) -> &mut PgConnection {
        match &mut self.conn {
            ConnectionHolder::Pooled(conn) => conn,
            ConnectionHolder::Direct(conn) => conn,
            ConnectionHolder::Transaction(conn) => conn,
        }
    }

    pub async fn commit(self) -> anyhow::Result<()> {
        if let ConnectionHolder::Transaction(transaction) = self.conn {
            transaction
                .commit()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to commit transaction: {}", e.to_string()))?;
        } else {
            panic!("StorageProcessor::commit can only be invoked after calling StorageProcessor::begin_transaction");
        }
        Ok(())
    }

    pub async fn rollback(self) -> anyhow::Result<()> {
        if let ConnectionHolder::Transaction(transaction) = self.conn {
            transaction.rollback().await.map_err(|e| {
                anyhow::anyhow!("Failed to rollback transaction: {}", e.to_string())
            })?;
        } else {
            panic!("StorageProcessor::rollback can only be invoked after calling StorageProcessor::begin_transaction");
        }
        Ok(())
    }
}
