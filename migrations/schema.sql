CREATE TABLE dinos (
    id uuid NOT NULL,
    name text,
    weight integer,
    diet text
);

ALTER TABLE ONLY dinos
    ADD CONSTRAINT dinos_pkey PRIMARY KEY (id);