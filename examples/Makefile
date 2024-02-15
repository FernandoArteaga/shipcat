minikube-create-arch:
	sudo -E minikube start --driver=none --kubernetes-version v1.16.2 --extra-config kubeadm.ignore-preflight-errors=SystemVerification

create-ns:
	kubectl create namespace apps

minikube:
	kubectl config set-context --cluster=minikube --user=minikube --namespace=apps minikube
	kubectl config use-context minikube
	$(MAKE) create-ns

minikube-cleanup:
	minikube delete --all

kind-create:
	kind create cluster --name shipcat --image "kindest/node:v1.19.16"

kind-cleanup:
	kind delete cluster --name shipcat

kind: kind-create
	kubectl config use-context kind-shipcat
	$(MAKE) create-ns

integrations:
	./integrations.sh

.PHONY: integrations kind minikube create-ns
