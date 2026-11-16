<!-- description: Deploy Orion for production: one instance with Docker, a Helm release on Kubernetes, or a Standard+ multi-node cluster with Redis. -->
<!-- type: hub -->
<!-- last_verified: 2026-11-16 -->

# Deploy Orion

One binary, four shapes. Each page here is one of them, and the cluster page explains what the others are configuring.

- [Deploy Orion Ext](./orion-ext.md) covers the Standard and Enterprise artifact: the licensed image, the mounted edition config and the reference HA topology. Community's image is the unmarked default on every other page.
- [Deploy with Docker](./docker.md) runs the image with a durable volume, a drain budget the orchestrator respects, and health probes. It also holds the reference HA Compose topology.
- [Deploy on Kubernetes](./kubernetes.md) installs the official Helm chart: N replicas, a pre-upgrade migration Job, surge rolling deploys, and a dedicated metrics listener.
- [Run a cluster](./cluster.md) is: what replicas share through Redis, how a deployment reaches every node, what stays per node, and why migrations are a deploy step.
