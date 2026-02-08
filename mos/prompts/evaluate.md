# State Evaluation

You are evaluating the current state of a software project workspace.

## Task
The user requested: {{task}}

## Instructions
Inspect the workspace thoroughly. Look at:
- What files exist (directories, source files, config files)
- Whether there is a plan or design document
- What modules/components have been implemented
- Whether tests exist and if they pass
- Whether the project builds/runs successfully

Then output a JSON object with exactly this structure:

```json
{
  "state": [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
  "summary": "brief description of current state",
  "modules": ["list", "of", "identified", "modules"],
  "errors": ["list of current errors or issues"],
  "next_priority": "what should be done next"
}
```

## State Vector Positions
- [0] has_plan: 1 if a clear implementation plan exists (in files or as structured comments)
- [1] has_structure: 1 if project structure is scaffolded (dirs, package.json/Cargo.toml, etc.)
- [2] modules_defined: 1 if distinct modules/components are identified with clear boundaries
- [3] module_impl: 1 if at least one module is fully implemented (not just scaffolded)
- [4] all_modules_impl: 1 if ALL identified modules are implemented
- [5] has_tests: 1 if test files exist with actual test cases
- [6] tests_pass: 1 if tests execute and pass (0 if they fail or don't exist)
- [7] has_integration: 1 if modules are wired together (imports, main entry point works)
- [8] integration_pass: 1 if the integrated project runs end-to-end successfully
- [9] verified: 1 if final verification confirms the project meets the original task

## Rules
- Be honest. If something is partial, mark it 0.
- Use the tools to actually check — read files, run tests, execute the project.
- Do NOT guess. Actually verify each state position.
- Output ONLY the JSON object, nothing else.
