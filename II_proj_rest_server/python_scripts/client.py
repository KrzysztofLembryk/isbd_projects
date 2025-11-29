#!/usr/bin/env python3
import requests
import concurrent.futures
import time

SERVER_URL = "http://localhost:8080"
TABLE_ID = "550e8400-e29b-41d4-a716-446655440000"
NUM_REQUESTS = 50

def send_request(request_id):
    url = f"{SERVER_URL}/table/{TABLE_ID}"
    start = time.time()
    try:
        response = requests.get(url)
        duration = time.time() - start
        print(f"Request {request_id} - Status: {response.status_code} - Time: {duration:.3f}s, RESPONSE MSG: {response.content}")
        return response.status_code
    except Exception as e:
        print(f"Request {request_id} - Error: {e}")
        return None

if __name__ == "__main__":
    print(f"Sending {NUM_REQUESTS} concurrent requests...")
    
    with concurrent.futures.ThreadPoolExecutor(max_workers=NUM_REQUESTS) as executor:
        futures = [executor.submit(send_request, i) for i in range(1, NUM_REQUESTS + 1)]
        results = [f.result() for f in concurrent.futures.as_completed(futures)]
    
    print(f"\nAll {NUM_REQUESTS} requests completed")
    print(f"Success: {sum(1 for r in results if r == 200)}")
    print(f"Errors: {sum(1 for r in results if r != 200)}")