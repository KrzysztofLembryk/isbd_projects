# Directory containing all test CSV files
CSV_DIR = "./csvs_for_tests/"

# ============================================================================
# CORRECT CSV FILES
# ============================================================================

# Employees table CSVs (20 rows, 3 columns: emp_id, name, salary)
OK_EMPLOYEES_WITH_HEADER = CSV_DIR + "ok_employees_with_header.csv"
OK_EMPLOYEES_NO_HEADER = CSV_DIR + "ok_employees_no_header.csv"

# ============================================================================
# ERROR CASE CSV FILES
# ============================================================================

# Wrong column name in header
WRONG_EMPLOYEES_WRONG_COLUMN_NAME = CSV_DIR + "wrong_employees_wrong_column_name.csv"

# Wrong number of columns (no header)
WRONG_EMPLOYEES_NO_HEADER_LESS_COLUMNS = CSV_DIR + "wrong_employees_no_header_less_columns.csv"
WRONG_EMPLOYEES_NO_HEADER_MORE_COLUMNS = CSV_DIR + "wrong_employees_no_header_more_columns.csv"

# Wrong number of columns (with header)
WRONG_EMPLOYEES_WITH_HEADER_LESS_COLUMNS = CSV_DIR + "wrong_employees_with_header_less_columns.csv"
WRONG_EMPLOYEES_WITH_HEADER_MORE_COLUMNS = CSV_DIR + "wrong_employees_with_header_more_columns.csv"

# Data type errors
WRONG_EMPLOYEES_STR_INSTEAD_OF_INT = CSV_DIR + "wrong_employees_str_instead_of_int.csv"

# Row value count errors
WRONG_EMPLOYEES_TOO_MANY_VALUES_IN_ROW = CSV_DIR + "wrong_employees_too_many_values_in_row.csv"
WRONG_EMPLOYEES_TOO_FEW_VALUES_IN_ROW = CSV_DIR + "wrong_employees_too_few_values_in_row.csv"

# Edge cases
WRONG_EMPLOYEES_ONLY_HEADER = CSV_DIR + "wrong_employees_only_header.csv"
WRONG_EMPLOYEES_EMPTY = CSV_DIR + "wrong_employees_empty.csv"
WRONG_EMPLOYEES_NON_EXISTENT_FILE = CSV_DIR + "this_file_does_not_exist.csv"