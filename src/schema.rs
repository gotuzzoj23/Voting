use diesel::{self, prelude::*};
use itertools::Itertools;
use rcir;



mod schema {
    table! {
        users {
            id -> Integer,
            username -> Text,
        }
    }

    table! {
        items {
            id -> Integer,
            title -> Text,
            body -> Text,
            done -> Bool,
        }
    }

    table! {
        votes (user_id, item_id) {
            user_id -> Integer,
            item_id -> Integer,
            ordinal -> Integer,
        }
    }
}

use self::schema::items;
use self::schema::users;
use self::schema::votes;
joinable!(votes -> users (user_id));
joinable!(votes -> items (item_id));
allow_tables_to_appear_in_same_query!(users, items, votes);

//#[table_name = "users"]
#[derive(Queryable, Debug)]
pub struct User {
    pub id: i32,
    pub username: String,
}

//#[table_name = "items"]
#[derive(Serialize, Queryable, Debug)]
pub struct Item {
    pub id: i32,
    pub title: String,
    pub body: String,
    pub done: bool,
}

//#[table_name = "votes"]
#[derive(Queryable, Insertable, Debug, Clone)]
pub struct Vote {
    pub user_id: i32,
    pub item_id: i32,
    pub ordinal: i32,
}

use self::schema::items::dsl::{items as all_items, done as item_done};
use self::schema::votes::dsl::{ordinal, user_id, item_id, votes as all_votes};
use self::schema::users::dsl::{users as all_users, username as users_uname};

#[derive(Deserialize)]
pub struct Ballot {
    pub votes: Vec<i32>,
}

#[derive(Insertable, FromForm)]
#[table_name = "users"]
pub struct NewUser {
    pub username: String,
}

impl NewUser {
    pub fn login(self, conn: &SqliteConnection) -> User {
        // Ensure that the user exist.
        let _ = diesel::insert_into(self::schema::users::table)
            .values(&self)
            .execute(conn);

        all_users
            .filter(users_uname.eq(&self.username))
            .get_result::<User>(conn)
            .unwrap()
    }
}

impl Item {
    pub fn all(conn: &SqliteConnection) -> Vec<Item> {
        all_items.load::<Item>(conn).unwrap()
    }

    pub fn for_user(uid: i32, conn: &SqliteConnection) -> Vec<(Item, bool)> {
        all_items
            .left_join(
                self::schema::votes::table
                    .on(user_id.eq(&uid).and(item_id.eq(self::schema::items::id))),
            )
            .filter(self::schema::items::done.eq(false))
            .order((user_id.desc(), ordinal.asc()))
            .select((self::schema::items::all_columns, ordinal.nullable()))
            .load::<(Item, Option<i32>)>(conn)
            .unwrap()
            .into_iter()
            .map(|(i, ord)| (i, ord.map(|_| true).unwrap_or(false)))
            .collect()
    }
}

impl Vote {
    pub fn run_election(conn: &SqliteConnection) -> Option<Item> {

        let votes = all_votes
            .inner_join(self::schema::items::table)
            .filter(item_done.eq(false))
            .order((user_id.asc(), ordinal.asc()))
            .select((user_id, item_id, ordinal))
            .load::<Vote>(conn)
            .unwrap();

        // the extra collections here are sad.
        let votes: Vec<Vec<_>> = votes
            .into_iter()
            .group_by(|v| v.user_id)
            .into_iter()
            .map(|(_, ballot)| ballot.into_iter().map(|v| v.item_id).collect())
            .collect();

        match rcir::run_election(&votes, rcir::MajorityMode::RemainingMajority).ok()? {
            rcir::ElectionResult::Winner(&iid) => {
                Some(all_items.find(iid).get_result::<Item>(conn).unwrap())
            },
            rcir::ElectionResult::Tie(iids) => {
                Some(all_items.find(iids[0]).get_result::<Item>(conn).unwrap())
            },
        }
    }

    /// Returns the number of affected rows: 1.
    pub fn save_ballot(uid: i32, ballot: Ballot, conn: &SqliteConnection)   {
        diesel::delete(all_votes.filter(user_id.eq(&uid)))
            .execute(conn)
            .unwrap();

            // i = ordinal # and iid = Item id in ballot votes 
            for (i, iid) in ballot.votes.into_iter().enumerate() {
                diesel::insert_into(self::schema::votes::table).values(Vote {
                    user_id: uid,
                    item_id: iid,
                    ordinal: i as i32,
                }).execute(conn).unwrap();
            }
    }

    // /// Returns the number of affected rows: 1.
    // pub async fn toggle_with_id(id: i32, conn: &DbConn) -> QueryResult<usize> {
    //     conn.run(move |c| {
    //         let task = all_tasks.find(id).get_result::<Task>(c)?;
    //         let new_status = !task.completed;
    //         let updated_task = diesel::update(all_tasks.find(id));
    //         updated_task.set(task_completed.eq(new_status)).execute(c)
    //     }).await
    // }

    // /// Returns the number of affected rows: 1.
    // pub async fn delete_with_id(id: i32, conn: &DbConn) -> QueryResult<usize> {
    //     conn.run(move |c| diesel::delete(all_tasks.find(id)).execute(c)).await
    // }

    // /// Returns the number of affected rows.
    // #[cfg(test)]
    // pub async fn delete_all(conn: &DbConn) -> QueryResult<usize> {
    //     conn.run(|c| diesel::delete(all_tasks).execute(c)).await
    // }
}
