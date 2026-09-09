# Specifications

## Overall

1. Write this in Rust
2. Make each component modular and stick to DRY principles.
3. Abstract concepts as necessary to allow for extensibility and such.

## Generator

1. Split it into two components: parser and .fit generator.
   1. the parser is responsible for parsing the input into a standard format that the program uses.
   2. the generator will take the standardized input and generate the .fit file.
2. Define a standardized format after researching what workouts look like written down.
3. Must be able to parse various formats - most of which will be defined later. Start with the swimdojo format.
4. This should be able to be used both as a standalone binary and as a library.
5. Must be able to handle various inputs, including file and stdin (if binary, otherwise just string)
6. Define terms used in workouts and generate a definition to follow - might make sense to generate an IDL.
7. Store workout formats as schemas.

## Scraper

1. Explore swimdojo website and get an idea of its layout as well as the format of the workouts. We should allow filtering the page and such
2. Make this extensible so that the scraper can scrape other sites in the future.
3. It should be able to infer a workout format from a site, generating a schema for the generator to consume.
