# Phase: Final Verification

## Task
The user requested: {{task}}

## Instructions
Perform final verification that the project meets all requirements. You must:

1. Re-read the original task
2. Run all tests — they must pass
3. Run the program with realistic inputs
4. Check that the output is correct
5. Verify error handling works
6. Check code quality (no obvious bugs, reasonable organization)

## Verification Checklist
- [ ] All tests pass
- [ ] Program runs end-to-end
- [ ] Output is correct for multiple inputs
- [ ] Errors are handled gracefully
- [ ] Code is reasonably organized
- [ ] The original task requirements are met

## Output
After verification, output a JSON summary:
```json
{
  "verified": true/false,
  "tests_passed": "X/Y",
  "issues": ["list of remaining issues if any"],
  "summary": "brief assessment"
}
```
