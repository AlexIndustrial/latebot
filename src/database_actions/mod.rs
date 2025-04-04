use std::sync::Arc;

use day::Day;
use mongodb::{bson::doc, options::ClientOptions, Client, Collection};

pub mod day;

pub type DatabaseService = Arc<DatabaseServiceInner>;

pub struct DatabaseServiceInner {
    collection: Collection<Day>,
}

impl DatabaseServiceInner {
    pub async fn new(connection_uri: &str) -> DatabaseService {
        let client_options = ClientOptions::parse(connection_uri)
            .await
            .expect("Failed to parse MongoDB options");
        let client =
            Client::with_options(client_options).expect("Failed to initialize MongoDB client");
        let db = client.database("latebot");
        let collection = db.collection::<Day>("days");

        let inner = DatabaseServiceInner { collection };

        Arc::new(inner)
    }

    pub async fn check_today_document(&self) -> Result<Day, mongodb::error::Error> {
        let now = mongodb::bson::DateTime::now();
        let today_start = mongodb::bson::DateTime::from_millis(
            now.timestamp_millis() - (now.timestamp_millis() % 86400000)
        );
        
        let filter = doc! {
            "date": today_start
        };
        
        match self.collection.find_one(filter, None).await.unwrap() {
            Some(day) => Ok(day),
            None => {
                let new_day = Day {
                    date: today_start,
                    votes_yes: Vec::new(),
                    votes_no: Vec::new(),
                };
                self.collection.insert_one(&new_day, None).await.unwrap();
                Ok(new_day)
            }
        }
    }

    pub async fn vote(&self, user_id: i64, vote_yes: bool) -> Result<(), mongodb::error::Error> {
        let now = mongodb::bson::DateTime::now();
        let today_start = mongodb::bson::DateTime::from_millis(
            now.timestamp_millis() - (now.timestamp_millis() % 86400000)
        );

        let filter = doc! {
            "date": today_start
        };

        // Определяем, какие поля обновлять в зависимости от голоса
        let (add_to_field, remove_from_field) = if vote_yes {
            ("votes_yes", "votes_no")
        } else {
            ("votes_no", "votes_yes")
        };

        // Обновляем документ: добавляем голос в нужный вектор и удаляем из противоположного
        let update = doc! {
            "$addToSet": {
                add_to_field: user_id
            },
            "$pull": {
                remove_from_field: user_id
            }
        };

        self.collection.update_one(filter, update, None).await?;
        Ok(())
    }

    pub async fn get_day_stats(&self, date: mongodb::bson::DateTime) -> Result<Day, mongodb::error::Error> {
        let filter = doc! {
            "date": date
        };
        
        match self.collection.find_one(filter, None).await? {
            Some(day) => Ok(day),
            None => Err(mongodb::error::Error::from(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Документ за указанную дату не найден"
            )))
        }
    }

    pub async fn get_total_late_days(&self) -> Result<i32, mongodb::error::Error> {
        let filter = doc! {
            "votes_yes": { "$exists": true, "$ne": [] }
        };
        
        let count = self.collection.count_documents(filter, None).await?;
        Ok(count as i32)
    }
}
