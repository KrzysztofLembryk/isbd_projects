#!/usr/bin/env python3

import csv
import random

def generate_large_csv():
    """
    Generate a CSV file with 3 columns and 500 rows.
    Columns: emp_id (INT64), name (VARCHAR), salary (INT64)
    Every 50 rows are identical (to test compression efficiency).
    """
    
    first_names = [
        "Alice", "Bob", "Charlie", "Diana", "Edward", "Fiona", "George", "Hannah",
        "Isaac", "Julia", "Kevin", "Laura", "Michael", "Nancy", "Oliver", "Patricia",
        "Quentin", "Rachel", "Samuel", "Tina", "Uma", "Victor", "Wendy", "Xavier",
        "Yvonne", "Zachary", "Aaron", "Bella", "Carter", "Daisy", "Ethan", "Faith"
    ]
    
    last_names = [
        "Smith", "Johnson", "Williams", "Brown", "Jones", "Garcia", "Miller", "Davis",
        "Rodriguez", "Martinez", "Hernandez", "Lopez", "Gonzalez", "Wilson", "Anderson",
        "Thomas", "Taylor", "Moore", "Jackson", "Martin", "Lee", "Walker", "Hall",
        "Allen", "Young", "King", "Wright", "Scott", "Green", "Baker", "Adams", "Nelson"
    ]
    
    filename = "../csvs_for_tests/large_employees_500.csv"
    
    with open(filename, 'w', newline='') as csvfile:
        writer = csv.writer(csvfile)
        
        # Write header
        writer.writerow(['emp_id', 'name', 'salary'])
        
        # Generate 10 unique patterns (each repeated 50 times)
        for pattern_num in range(10):
            # Create one pattern
            name = f"{random.choice(first_names)} {random.choice(last_names)}"
            salary = random.randint(30000, 150000)
            
            # Repeat this pattern 50 times
            for i in range(50):
                emp_id = pattern_num * 50 + i + 1  # Sequential emp_id: 1-500
                writer.writerow([emp_id, name, salary])
    
    print(f"✓ Generated {filename} with 500 rows")
    print(f"  Columns: emp_id (INT64), name (VARCHAR), salary (INT64)")
    print(f"  Pattern: 10 unique row patterns, each repeated 50 times")
    print(f"  (Good for testing compression efficiency)")

if __name__ == "__main__":
    generate_large_csv()