CREATE TABLE guilds(
  guildID BIGINT NOT NULL UNIQUE PRIMARY KEY,
  channel BIGINT NOT NULL
);

CREATE INDEX on guilds(guildID);

CREATE TABLE users(
  id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  guildID BIGINT NOT NULL REFERENCES guilds(guildID),
  userID BIGINT NOT NULL,
  praise TEXT NOT NULL,
  praiseName TEXT NOT NULL,
  timezone INTERVAL NOT NULL
);

CREATE INDEX on users(guildID, userID);
CREATE INDEX on users(id);

CREATE TABLE schedule(
  id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  guildID BIGINT NOT NULL REFERENCES guilds(guildID), 
  userID BIGINT NOT NULL REFERENCES users(id),
  task TEXT NOT NULL,
  taskSecondary TEXT NOT NULL,
  interval INTERVAL NOT NULL,
  created  TIMESTAMP NOT NULL DEFAULT (NOW() at time zone 'utc'),
  nextRun TIMESTAMP NOT NULL
);

CREATE INDEX on schedule(nextRun);
CREATE INDEX on schedule(guildID, userID);
CREATE INDEX on schedule(id);
