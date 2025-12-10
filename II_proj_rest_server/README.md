# Docker setup

```bash
# build image
docker build -t rest-server .

# run docker
docker run -p 8080:8080 -v /path_to_csv_files:/data --name my-rest-server rest-server

# if we want our db to have persistent data, like metadata file
# between shutdowns, we should also mount folder in which this 
# data will be stored
docker run -p 8080:8080 -v ./path_to_csv_files:/data -v ./path_to_db_data:/app/db_data --name my-rest-server rest-server


# to stop runnign container
docker stop my-rest-server

# before running it again we need to remove it
docker rm my-rest-server
```

# Building locally
```bash
# Need to have cargo pre-installed
# in folder with Cargo.toml run
cargo run
```

# Tests
- To run python tests one need to change 'FOR_TESTS_DO_LONG_QUERY_EXECUTION' variable
in constants.rs. This variable simulates long query execution, so that we don't need to use big csv files.
Not pretty but works.
