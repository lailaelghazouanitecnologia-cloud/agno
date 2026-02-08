# Phase: Planning

## Task
The user requested: {{task}}

## Instructions
Create a detailed implementation plan for this project. You must:

1. Analyze what the task requires — break it into components
2. Identify the modules/components needed (each should be independent and testable)
3. Define the interface between modules
4. Choose the technology stack and project structure
5. Write the plan to a file called `PLAN.md` in the workspace root

## Plan Format (write to PLAN.md)
```markdown
# Implementation Plan

## Overview
Brief description of what we're building.

## Modules
### Module: {name}
- **Purpose**: what this module does
- **Files**: list of files to create
- **Interface**: exported functions/classes
- **Dependencies**: what it depends on
- **Test strategy**: how to test it

### Module: {name}
...

## Integration
How modules connect together.

## Entry Point
What the main program does and how to run it.

## Test Strategy
How to verify the complete system works.
```

## Rules
- Be thorough. Each module should be self-contained.
- Think about error handling and edge cases.
- The plan should be detailed enough that each module can be implemented independently.
- Create the PLAN.md file using write_file. Do not just output text.
- Also create the basic project config (package.json, tsconfig.json, Cargo.toml, etc.)
