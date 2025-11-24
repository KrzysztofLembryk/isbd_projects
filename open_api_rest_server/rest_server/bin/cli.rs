//! CLI tool driving the API client
use anyhow::{anyhow, Context, Result};
use clap::Parser;
use dialoguer::Confirm;
use log::{debug, info};
// models may be unused if all inputs are primitive types
#[allow(unused_imports)]
use openapi_client::{
    models, ApiNoContext, Client, ContextWrapperExt,
    GetQueriesResponse,
    GetSystemInfoResponse,
    SubmitQueryResponse,
    GetQueryByIdResponse,
    GetQueryErrorResponse,
    GetQueryResultResponse,
    CreateTableResponse,
    GetTablesResponse,
    DeleteTableResponse,
    GetTableByIdResponse,
};
use simple_logger::SimpleLogger;
use swagger::{AuthData, ContextBuilder, EmptyContext, Push, XSpanIdString};

type ClientContext = swagger::make_context_ty!(
    ContextBuilder,
    EmptyContext,
    Option<AuthData>,
    XSpanIdString
);

#[derive(Parser, Debug)]
#[clap(
    name = "MIMUW ISBD database system",
    version = "1.0.0",
    about = "CLI access to MIMUW ISBD database system"
)]
struct Cli {
    #[clap(subcommand)]
    operation: Operation,

    /// Address or hostname of the server hosting this API, including optional port
    #[clap(short = 'a', long, default_value = "http://localhost")]
    server_address: String,

    /// Path to the client private key if using client-side TLS authentication
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "ios")))]
    #[clap(long, requires_all(&["client_certificate", "server_certificate"]))]
    client_key: Option<String>,

    /// Path to the client's public certificate associated with the private key
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "ios")))]
    #[clap(long, requires_all(&["client_key", "server_certificate"]))]
    client_certificate: Option<String>,

    /// Path to CA certificate used to authenticate the server
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "ios")))]
    #[clap(long)]
    server_certificate: Option<String>,

    /// If set, write output to file instead of stdout
    #[clap(short, long)]
    output_file: Option<String>,

    #[command(flatten)]
    verbosity: clap_verbosity_flag::Verbosity,

    /// Don't ask for any confirmation prompts
    #[allow(dead_code)]
    #[clap(short, long)]
    force: bool,
}

#[derive(Parser, Debug)]
enum Operation {
    /// Get list of queries (optional in project 3, but useful). Use those IDs to get details by calling /query endpoint.
    GetQueries {
    },
    /// Get basic information about the system (e.g. version, uptime, etc.)
    GetSystemInfo {
    },
    /// Submit new query for execution
    SubmitQuery {
        /// Used to submit a new query for execution
        #[clap(value_parser = parse_json::<models::ExecuteQueryRequest>)]
        execute_query_request: models::ExecuteQueryRequest,
    },
    /// Get detailed status of selected query
    GetQueryById {
        /// ID of selected Query
        query_id: String,
    },
    /// Get error of selected query (will be available only for queries in FAILED state)
    GetQueryError {
        /// ID of selected Query
        query_id: String,
    },
    /// Get result of selected query (will be available only for SELECT queries after they are completed)
    GetQueryResult {
        /// ID of selected Query
        query_id: String,
        /// Used to get result of a query
        #[clap(value_parser = parse_json::<models::GetQueryResultRequest>)]
        get_query_result_request: Option<models::GetQueryResultRequest>,
    },
    /// Create new table in database
    CreateTable {
        /// Used to create a new table
        #[clap(value_parser = parse_json::<models::TableSchema>)]
        table_schema: models::TableSchema,
    },
    /// Get list of tables with their accompaning IDs. Use those IDs to get details by calling /table endpoint.
    GetTables {
    },
    /// Delete selected table from database
    DeleteTable {
        /// ID of selected Table
        table_id: String,
    },
    /// Get detailed description of selected table
    GetTableById {
        /// ID of selected Table
        table_id: String,
    },
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "ios")))]
fn create_client(args: &Cli, context: ClientContext) -> Result<Box<dyn ApiNoContext<ClientContext>>> {
    if args.client_certificate.is_some() {
        debug!("Using mutual TLS");
        let client = Client::try_new_https_mutual(
            &args.server_address,
            args.server_certificate.clone().unwrap(),
            args.client_key.clone().unwrap(),
            args.client_certificate.clone().unwrap(),
        )
        .context("Failed to create HTTPS client")?;
        Ok(Box::new(client.with_context(context)))
    } else if args.server_certificate.is_some() {
        debug!("Using TLS with pinned server certificate");
        let client =
            Client::try_new_https_pinned(&args.server_address, args.server_certificate.clone().unwrap())
                .context("Failed to create HTTPS client")?;
        Ok(Box::new(client.with_context(context)))
    } else {
        debug!("Using client without certificates");
        let client =
            Client::try_new(&args.server_address).context("Failed to create HTTP(S) client")?;
        Ok(Box::new(client.with_context(context)))
    }
}

#[cfg(any(target_os = "macos", target_os = "windows", target_os = "ios"))]
fn create_client(args: &Cli, context: ClientContext) -> Result<Box<dyn ApiNoContext<ClientContext>>> {
    let client =
        Client::try_new(&args.server_address).context("Failed to create HTTP(S) client")?;
    Ok(Box::new(client.with_context(context)))
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    if let Some(log_level) = args.verbosity.log_level() {
        SimpleLogger::new().with_level(log_level.to_level_filter()).init()?;
    }

    debug!("Arguments: {:?}", &args);

    let auth_data: Option<AuthData> = None;

    #[allow(trivial_casts)]
    let context = swagger::make_context!(
        ContextBuilder,
        EmptyContext,
        auth_data,
        XSpanIdString::default()
    );

    let client = create_client(&args, context)?;

    let result = match args.operation {
        Operation::GetQueries {
        } => {
            info!("Performing a GetQueries request");

            let result = client.get_queries(
            ).await?;
            debug!("Result: {:?}", result);

            match result {
                GetQueriesResponse::ArrayOfQueriesSubmittedToTheSystem
                (body)
                => "ArrayOfQueriesSubmittedToTheSystem\n".to_string()
                   +
                    &serde_json::to_string_pretty(&body)?,
            }
        }
        Operation::GetSystemInfo {
        } => {
            info!("Performing a GetSystemInfo request");

            let result = client.get_system_info(
            ).await?;
            debug!("Result: {:?}", result);

            match result {
                GetSystemInfoResponse::BasicInformationAboutTheSystem
                (body)
                => "BasicInformationAboutTheSystem\n".to_string()
                   +
                    &serde_json::to_string_pretty(&body)?,
            }
        }
        Operation::SubmitQuery {
            execute_query_request,
        } => {
            info!("Performing a SubmitQuery request");

            let result = client.submit_query(
                execute_query_request,
            ).await?;
            debug!("Result: {:?}", result);

            match result {
                SubmitQueryResponse::QueryHasBeenCreatedSuccessfully
                (body)
                => "QueryHasBeenCreatedSuccessfully\n".to_string()
                   +
                    &serde_json::to_string_pretty(&body)?,
                SubmitQueryResponse::ResponseUsedWhenMoreProblemsCanOccurInTheSystemWhenProcessingRequest
                (body)
                => "ResponseUsedWhenMoreProblemsCanOccurInTheSystemWhenProcessingRequest\n".to_string()
                   +
                    &serde_json::to_string_pretty(&body)?,
            }
        }
        Operation::GetQueryById {
            query_id,
        } => {
            info!("Performing a GetQueryById request on {:?}", (
                &query_id
            ));

            let result = client.get_query_by_id(
                query_id,
            ).await?;
            debug!("Result: {:?}", result);

            match result {
                GetQueryByIdResponse::DetailedQueryDescription
                (body)
                => "DetailedQueryDescription\n".to_string()
                   +
                    &serde_json::to_string_pretty(&body)?,
                GetQueryByIdResponse::GenericError
                (body)
                => "GenericError\n".to_string()
                   +
                    &serde_json::to_string_pretty(&body)?,
            }
        }
        Operation::GetQueryError {
            query_id,
        } => {
            info!("Performing a GetQueryError request on {:?}", (
                &query_id
            ));

            let result = client.get_query_error(
                query_id,
            ).await?;
            debug!("Result: {:?}", result);

            match result {
                GetQueryErrorResponse::ResponseUsedWhenMoreProblemsCanOccurInTheSystemWhenProcessingRequest
                (body)
                => "ResponseUsedWhenMoreProblemsCanOccurInTheSystemWhenProcessingRequest\n".to_string()
                   +
                    &serde_json::to_string_pretty(&body)?,
                GetQueryErrorResponse::GenericError
                (body)
                => "GenericError\n".to_string()
                   +
                    &serde_json::to_string_pretty(&body)?,
                GetQueryErrorResponse::GenericError_2
                (body)
                => "GenericError_2\n".to_string()
                   +
                    &serde_json::to_string_pretty(&body)?,
            }
        }
        Operation::GetQueryResult {
            query_id,
            get_query_result_request,
        } => {
            info!("Performing a GetQueryResult request on {:?}", (
                &query_id
            ));

            let result = client.get_query_result(
                query_id,
                get_query_result_request,
            ).await?;
            debug!("Result: {:?}", result);

            match result {
                GetQueryResultResponse::ResultOfSelectedQuery
                (body)
                => "ResultOfSelectedQuery\n".to_string()
                   +
                    &serde_json::to_string_pretty(&body)?,
                GetQueryResultResponse::GenericError
                (body)
                => "GenericError\n".to_string()
                   +
                    &serde_json::to_string_pretty(&body)?,
                GetQueryResultResponse::GenericError_2
                (body)
                => "GenericError_2\n".to_string()
                   +
                    &serde_json::to_string_pretty(&body)?,
            }
        }
        Operation::CreateTable {
            table_schema,
        } => {
            info!("Performing a CreateTable request");

            let result = client.create_table(
                table_schema,
            ).await?;
            debug!("Result: {:?}", result);

            match result {
                CreateTableResponse::TableCreatedSuccessfully
                (body)
                => "TableCreatedSuccessfully\n".to_string()
                   +
                    &serde_json::to_string_pretty(&body)?,
                CreateTableResponse::ResponseUsedWhenMoreProblemsCanOccurInTheSystemWhenProcessingRequest
                (body)
                => "ResponseUsedWhenMoreProblemsCanOccurInTheSystemWhenProcessingRequest\n".to_string()
                   +
                    &serde_json::to_string_pretty(&body)?,
            }
        }
        Operation::GetTables {
        } => {
            info!("Performing a GetTables request");

            let result = client.get_tables(
            ).await?;
            debug!("Result: {:?}", result);

            match result {
                GetTablesResponse::ArrayOfTablesInDatabase
                (body)
                => "ArrayOfTablesInDatabase\n".to_string()
                   +
                    &serde_json::to_string_pretty(&body)?,
            }
        }
        Operation::DeleteTable {
            table_id,
        } => {
            prompt(args.force, "This will delete the given entry, are you sure?")?;
            info!("Performing a DeleteTable request on {:?}", (
                &table_id
            ));

            let result = client.delete_table(
                table_id,
            ).await?;
            debug!("Result: {:?}", result);

            match result {
                DeleteTableResponse::TableHasBeenDeletedSuccessfully
                => "TableHasBeenDeletedSuccessfully\n".to_string()
                    ,
                DeleteTableResponse::GenericError
                (body)
                => "GenericError\n".to_string()
                   +
                    &serde_json::to_string_pretty(&body)?,
            }
        }
        Operation::GetTableById {
            table_id,
        } => {
            info!("Performing a GetTableById request on {:?}", (
                &table_id
            ));

            let result = client.get_table_by_id(
                table_id,
            ).await?;
            debug!("Result: {:?}", result);

            match result {
                GetTableByIdResponse::DetailedTableDescription
                (body)
                => "DetailedTableDescription\n".to_string()
                   +
                    &serde_json::to_string_pretty(&body)?,
                GetTableByIdResponse::GenericError
                (body)
                => "GenericError\n".to_string()
                   +
                    &serde_json::to_string_pretty(&body)?,
            }
        }
    };

    if let Some(output_file) = args.output_file {
        std::fs::write(output_file, result)?
    } else {
        println!("{}", result);
    }
    Ok(())
}

fn prompt(force: bool, text: &str) -> Result<()> {
    if force || Confirm::new().with_prompt(text).interact()? {
        Ok(())
    } else {
        Err(anyhow!("Aborting"))
    }
}

// May be unused if all inputs are primitive types
#[allow(dead_code)]
fn parse_json<T: serde::de::DeserializeOwned>(json_string: &str) -> Result<T> {
    serde_json::from_str(json_string).map_err(|err| anyhow!("Error parsing input: {}", err))
}
