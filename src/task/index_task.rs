use crate::cluster::{Cluster};
use std::sync::Arc;
use crate::errors::{ErrorKind, Result};
use std::time::{Duration};
use crate::task::{Status, Task};


/// Struct for querying index creation status
#[derive(Debug, Clone)]
pub struct IndexTask {
    cluster: Arc<Cluster>,
    namespace: String,
    index_name: String
}


static SUCCESS_PATTERN: &'static str = "load_pct=";
static FAIL_PATTERN_201: &'static str = "FAIL:201";
static FAIL_PATTERN_203: &'static str = "FAIL:203";
static DELMITER: &'static str = ";";

impl IndexTask {
    /// Initializes IndexTask from client, creation should only be expose to Client
    pub fn new(cluster: Arc<Cluster>, namespace: String, index_name: String) -> Self {
        IndexTask {
        	cluster: cluster,
        	namespace: namespace,
            index_name: index_name
        }
    }

    fn build_command(namespace: String, index_name: String) -> String {
        return format!("{}{}{}{}", "sindex/", namespace, "/", index_name);
    }

	fn parse_response(response: &str) -> Result<Status> {
        match response.find(SUCCESS_PATTERN) {
            None => {
                match (response.find(FAIL_PATTERN_201), response.find(FAIL_PATTERN_203)) {
                    (None, None) => bail!(ErrorKind::BadResponse(format!("Code 201 and 203 missing. Response: {}", response))),
                    (_, _) => return Ok(Status::NotFound)
                }
            },
            Some(pattern_index) => {
                let percent_begin = pattern_index + SUCCESS_PATTERN.len();

                let percent_end = match response[percent_begin..].find(DELMITER) {
                    None =>  bail!(ErrorKind::BadResponse(format!("delimiter missing in response. Response: {}", response))),
                    Some(percent_end) => percent_end
                };
                let percent_str = &response[percent_begin..percent_begin+percent_end];
                if percent_str.parse::<isize>().unwrap() != 100 {
                    return Ok(Status::InProgress);
                } else {
                    return Ok(Status::Complete);
                }
            }
        }
    }
}

impl Task for IndexTask {
    /// Query the status of index creation across all nodes
    fn query_status(&self) -> Result<Status> {
        let nodes = self.cluster.nodes();

        if nodes.len() == 0 {
            bail!(ErrorKind::Connection("No connected node".to_string()))
        }

        for node in nodes.iter() {
            let command = &IndexTask::build_command(self.namespace.to_owned(), self.index_name.to_owned());
            let response = node.info(
                Some(self.cluster.client_policy().timeout.unwrap()),
                &[&command[..]]
            )?;

            if !response.contains_key(command) {
                return Ok(Status::NotFound);
            }

            match IndexTask::parse_response(&response[command]) {
                Ok(Status::NotFound) => return Ok(Status::NotFound),
                Ok(Status::InProgress) => return Ok(Status::InProgress),
                error => return error
            }
        }
        return Ok(Status::Complete);
    }

    fn get_timeout(&self) -> Result<Duration> {
        match self.cluster.client_policy().timeout {
            Some(duration) => {
                return Ok(duration);
            }
            _ => {
                bail!(ErrorKind::InvalidArgument("Timeout missing in client policy".to_string()))
            }
            
        }
    }
}






