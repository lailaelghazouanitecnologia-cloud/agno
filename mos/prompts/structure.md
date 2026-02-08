# Phase: Project Structure

## Task
The user requested: {{task}}

## Current Plan
{{plan}}

## Instructions
Create the project structure based on the plan. You must:

1. Read the PLAN.md file to understand the architecture
2. Create all directories needed
3. Create configuration files (package.json, tsconfig.json, etc.)
4. Create skeleton/stub files for each module with:
   - Correct exports/imports
   - Interface signatures (empty implementations)
   - TODO comments marking what needs to be filled in
5. Ensure the project can build (even if with empty implementations)

## Rules
- Create ALL files mentioned in the plan
- Every module should have its file(s) created
- Use write_file for every file
- After creating the structure, verify it builds (npm install + build, or cargo build)
