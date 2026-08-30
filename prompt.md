# Prelude
This file is your file that you are free to edit. DO NOT EDIT above the "OKAY TO EDIT BELOW THIS" line. You are free to do anything to the file below the line. Use this to keep track of progress, knowledge, and manage work. If you think there is an edit i should make ABOVE the line, put it in a file called prompt-edits.md. At each turn, I will review and update this section as necessary.

# Intro
I want to work on a program which has two major components:

1. A program which takes workouts written in a certain format, parses it, and generates a .fit file as an end product. Referred to as generator below.
2. a program which can scrape the workouts from https://www.swimdojo.com/workouts. referred to as scraper below.

# Guidelines
1. Create atomic commits that compile and function.
2. Break down the work into tasks and keep track of them in this file
3. it's okay to challenge what i tell you - i may be wrong. 
4. always give me data backed decisions. point to evidence.
5. keep track of the turn below.

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

OKAY TO EDIT BELOW THIS

TURN: 

# Plan

# Tasks

# Bugs/Issues

# References
