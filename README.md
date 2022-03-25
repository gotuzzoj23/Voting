# Voting
Ranked Choice voting app

Simple Ranked Choice voting app build on Rust using Rocket and Diesel crates. Followed Jon Gjengset video as a learning opportunity.

To enter new candidates into the poll use the following:

`INSERT INTO items (title, body) VALUES ("New candidate");`

Once polling is complete for a candidate you can mark them as 'done':

`UPDATE items SET done = true WHERE id = <ID>;`

The <ID> can be found by the following command:
  
`SELECT id, title FROM items WHERE done = false;`
