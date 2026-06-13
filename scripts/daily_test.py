import json
import subprocess
import os
import sys
from datetime import datetime

# Paths
BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SCRIPTS_DIR = os.path.join(BASE_DIR, 'scripts')
URLS_FILE = os.path.join(SCRIPTS_DIR, 'test_urls.json')
STATE_FILE = os.path.join(SCRIPTS_DIR, 'test_state.json')
RESULTS_DIR = os.path.join(SCRIPTS_DIR, 'results')

def load_json(path, default):
    if not os.path.exists(path):
        return default
    with open(path, 'r', encoding='utf-8') as f:
        return json.load(f)

def save_json(path, data):
    with open(path, 'w', encoding='utf-8') as f:
        json.dump(data, f, indent=4)

def run_test():
    # Ensure results directory exists
    if not os.path.exists(RESULTS_DIR):
        os.makedirs(RESULTS_DIR)

    # Load data
    urls_data = load_json(URLS_FILE, [])
    if not urls_data:
        print("Error: No URLs found in test_urls.json")
        return

    state = load_json(STATE_FILE, {"last_index": -1, "total_runs": 0})

    # Calculate next 5 sites
    start_index = (state["last_index"] + 1) % len(urls_data)
    sites_to_test = []
    for i in range(5):
        idx = (start_index + i) % len(urls_data)
        sites_to_test.append(urls_data[idx])

    print(f"--- MangoFetch Daily Test: {datetime.now().strftime('%Y-%m-%d')} ---")
    print(f"Testing {len(sites_to_test)} sites starting from index {start_index}\n")

    results = {
        "date": datetime.now().isoformat(),
        "tests": []
    }

    for site in sites_to_test:
        print(f"Testing {site['name']}...")
        print(f"URL: {site['url']}")

        # Run mangofetch-cli download
        # We use 'cargo run -p mangofetch-cli -- download <url> --output <results_dir>'
        try:
            # Create a temp output dir for this site to avoid clutter
            site_output_dir = os.path.join(RESULTS_DIR, site['name'].replace('/', '_'))
            if not os.path.exists(site_output_dir):
                os.makedirs(site_output_dir)

            cmd = [
                "cargo", "run", "-p", "mangofetch-cli", "--",
                "download", site['url'],
                "--output", site_output_dir
            ]

            # Using subprocess.run to capture output
            process = subprocess.run(
                cmd,
                cwd=BASE_DIR,
                capture_output=True,
                text=True,
                timeout=300 # 5 minutes timeout
            )

            success = process.returncode == 0
            status = "SUCCESS" if success else "FAILED"

            print(f"Status: {status}")
            if not success:
                print(f"Error: {process.stderr[:200]}...")

            results["tests"].append({
                "name": site["name"],
                "url": site["url"],
                "status": status,
                "exit_code": process.returncode,
                "error": process.stderr if not success else ""
            })

        except subprocess.TimeoutExpired:
            print("Status: TIMEOUT")
            results["tests"].append({
                "name": site["name"],
                "url": site["url"],
                "status": "TIMEOUT",
                "error": "Execution exceeded timeout"
            })
        except Exception as e:
            print(f"Status: ERROR ({str(e)})")
            results["tests"].append({
                "name": site["name"],
                "url": site["url"],
                "status": "ERROR",
                "error": str(e)
            })
        print("-" * 40)

    # Update state
    state["last_index"] = (start_index + 4) % len(urls_data)
    state["total_runs"] += 1
    state["last_run_date"] = datetime.now().isoformat()
    save_json(STATE_FILE, state)

    # Save results
    result_filename = f"test_result_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
    save_json(os.path.join(RESULTS_DIR, result_filename), results)

    print(f"\nTest run complete. Summary saved to {result_filename}")

if __name__ == "__main__":
    run_test()
