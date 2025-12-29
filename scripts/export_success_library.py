import os
import json
import psycopg2
from psycopg2.extras import RealDictCursor
from dotenv import load_dotenv

def export_library():
    load_dotenv()
    db_url = os.getenv("DATABASE_URL")
    if not db_url:
        print("❌ DATABASE_URL not found in .env")
        return

    output_file = "data/success_library_export.jsonl"
    os.makedirs("data", exist_ok=True)

    try:
        conn = psycopg2.connect(db_url)
        with conn.cursor(cursor_factory=RealDictCursor) as cur:
            cur.execute("SELECT * FROM success_stories ORDER BY timestamp DESC")
            rows = cur.fetchall()
            
            with open(output_file, "w") as f:
                for row in rows:
                    # Convert Decimals or other non-serializable objects if necessary
                    f.write(json.dumps(row, default=str) + "\n")
            
            print(f"✅ Exported {len(rows)} stories to {output_file}")
    except Exception as e:
        print(f"❌ Export failed: {e}")
    finally:
        if 'conn' in locals():
            conn.close()

if __name__ == "__main__":
    export_library()
