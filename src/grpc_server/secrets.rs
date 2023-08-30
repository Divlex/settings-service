use std::pin::Pin;

use futures_core::Stream;

use super::server::GrpcService;
use crate::app_ctx::SecretsValueReader;
use crate::secrets_grpc::secrets_server::Secrets;
use crate::secrets_grpc::*;

#[tonic::async_trait]
impl Secrets for GrpcService {
    type GetAllStream =
        Pin<Box<dyn Stream<Item = Result<SecretListItem, tonic::Status>> + Send + Sync + 'static>>;

    async fn get_all(
        &self,
        _: tonic::Request<()>,
    ) -> Result<tonic::Response<Self::GetAllStream>, tonic::Status> {
        if let Some(items) = crate::operations::get_all_secrets(&self.app).await {
            let mut result = Vec::with_capacity(items.len());
            for item in items {
                let templates_amount =
                    crate::operations::secrets::get_used_secret_amount_by_template(
                        &self.app,
                        item.get_secret_name(),
                    )
                    .await;

                let secrets_amount = crate::operations::secrets::get_used_secret_amount_by_secret(
                    &self.app,
                    item.get_secret_name(),
                )
                .await;

                result.push(SecretListItem {
                    name: item.row_key.clone(),
                    level: item.level.unwrap_or(0) as i32,
                    created: item.create_date.clone(),
                    updated: item.last_update_date.clone(),
                    used_by_secrets: secrets_amount as i32,
                    used_by_templates: templates_amount as i32,
                });
            }
            my_grpc_extensions::grpc_server::send_vec_to_stream(result, |item| item).await
        } else {
            my_grpc_extensions::grpc_server::send_vec_to_stream(vec![], |item| item).await
        }
    }

    async fn get(
        &self,
        request: tonic::Request<GetSecretRequest>,
    ) -> Result<tonic::Response<SecretModel>, tonic::Status> {
        let request = request.into_inner();

        let result = self.app.get_secret_value(&request.name).await;

        let result = match result {
            Some(value) => SecretModel {
                name: request.name,
                value: value.content,
                level: value.level as i32,
            },
            None => SecretModel {
                name: "".to_string(),
                value: "".to_string(),
                level: 0,
            },
        };

        Ok(tonic::Response::new(result))
    }

    async fn get_templates_usage(
        &self,
        request: tonic::Request<GetTemplatesUsageRequest>,
    ) -> Result<tonic::Response<GetTemplatesUsageResponse>, tonic::Status> {
        let request = request.into_inner();

        let result =
            crate::operations::get_secret_usage_in_templates(&self.app, &request.name).await;

        let templates = result
            .into_iter()
            .map(|x| TemplateUsageModel {
                env: x.env,
                name: x.name,
                yaml: x.yaml,
            })
            .collect();

        Ok(tonic::Response::new(GetTemplatesUsageResponse {
            templates,
        }))
    }

    async fn get_secrets_usage(
        &self,
        request: tonic::Request<GetSecretsUsageRequest>,
    ) -> Result<tonic::Response<GetSecretsUsageResponse>, tonic::Status> {
        let request = request.into_inner();

        let result = crate::operations::get_secret_usage_in_secrets(&self.app, &request.name).await;

        let secrets = result
            .into_iter()
            .map(|x| SecretUsageModel {
                name: x.name,
                value: x.value,
            })
            .collect();

        Ok(tonic::Response::new(GetSecretsUsageResponse { secrets }))
    }

    async fn ping(&self, _: tonic::Request<()>) -> Result<tonic::Response<()>, tonic::Status> {
        Ok(tonic::Response::new(()))
    }
}
