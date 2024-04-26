use futures_util::StreamExt;
use mongodb::{
    bson::{doc, Bson},
    options::FindOneOptions,
    Collection,
};
use serde::{Deserialize, Serialize};
use tokio::fs;

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum Platform {
    Android,
    Ios,
    Full,
}

impl From<Platform> for Bson {
    fn from(value: Platform) -> Self {
        Self::String(format!("{:?}", value))
    }
}

skip_serialize_none! {
    UpdatePkg {
        pub project_name: String,
        pub pkg_file_name: Option<String>,
        pub pkg_link: Option<String>,
        pub app_version: String,
        pub version: u32,
        pub mandatory: bool,
        pub update_log: Option<String>,
        pub full_pkg: bool,
        pub platform: Platform,
    }
}

impl UpdatePkg {
    fn get_collection(db: &mongodb::Database) -> Collection<Self> {
        db.collection("pkg")
    }

    pub async fn create_index(db: &mongodb::Database) -> anyhow::Result<()> {
        let collection = Self::get_collection(db);
        let index_model = mongodb::IndexModel::builder()
            .keys(mongodb::bson::doc! { "projectName": 1, "appVersion": -1 })
            .build();
        collection.create_index(index_model, None).await?;
        Ok(())
    }

    pub async fn save(&self, db: &mongodb::Database) -> anyhow::Result<()> {
        let collection = Self::get_collection(db);
        collection.insert_one(self, None).await?;
        Ok(())
    }

    pub async fn find_new_version(
        project_name: &str,
        platform: &Platform,
        db: &mongodb::Database,
    ) -> anyhow::Result<Option<Self>> {
        let collection = Self::get_collection(db);
        let filter = mongodb::bson::doc! {
            "projectName": project_name,
            "platform": platform,
        };
        let find_options = FindOneOptions::builder()
            .sort(doc! { "appVersion": -1 })
            .build();
        let result = collection.find_one(filter, find_options).await?;
        Ok(result)
    }

    pub async fn find_by_version(
        project_name: &str,
        platform: &Platform,
        app_version: &str,
        db: &mongodb::Database,
    ) -> anyhow::Result<Option<Self>> {
        let collection = Self::get_collection(db);
        let filter = mongodb::bson::doc! {
            "projectName": project_name,
            "appVersion": app_version,
            "platform": platform,
        };
        let result = collection.find_one(filter, None).await?;
        Ok(result)
    }

    pub async fn remove(&self, db: &mongodb::Database) -> anyhow::Result<()> {
        let collection = Self::get_collection(db);
        let filter = mongodb::bson::doc! {
            "projectName": &self.project_name,
            "appVersion": &self.app_version,
        };
        let mut items = collection.find(filter.clone(), None).await?;
        while let Some(Ok(item)) = items.next().await {
            item.del_file().await?;
        }
        collection.delete_many(filter, None).await?;
        Ok(())
    }

    pub async fn del_file(&self) -> anyhow::Result<()> {
        let Some(pkg_file_name) = &self.pkg_file_name else {
            return Ok(());
        };
        let path = format!("./pkgs/{}", pkg_file_name);
        if fs::try_exists(&path).await.unwrap_or(false) {
            fs::remove_file(path).await?;
        }
        Ok(())
    }
}
