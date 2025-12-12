package v1alpha1

import (
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
)

// LumaClusterSpec defines the desired state of LumaCluster
type LumaClusterSpec struct {
	// Replicas is the number of LumaDB nodes
	Replicas int32 `json:"replicas,omitempty"`
	// Image is the container image to use
	Image string `json:"image,omitempty"`
	// StorageSize is the size of the PVC (e.g. "10Gi")
	StorageSize string `json:"storageSize,omitempty"`
}

// LumaClusterStatus defines the observed state of LumaCluster
type LumaClusterStatus struct {
	// ActiveNodes is the number of healthy nodes
	ActiveNodes int32 `json:"activeNodes"`
	// Phase is the current state (Initializing, Running, Failed)
	Phase string `json:"phase"`
}

// +kubebuilder:object:root=true
// +kubebuilder:subresource:status

// LumaCluster is the Schema for the lumaclusters API
type LumaCluster struct {
	metav1.TypeMeta   `json:",inline"`
	metav1.ObjectMeta `json:"metadata,omitempty"`

	Spec   LumaClusterSpec   `json:"spec,omitempty"`
	Status LumaClusterStatus `json:"status,omitempty"`
}

// +kubebuilder:object:root=true

// LumaClusterList contains a list of LumaCluster
type LumaClusterList struct {
	metav1.TypeMeta `json:",inline"`
	metav1.ListMeta `json:"metadata,omitempty"`
	Items           []LumaCluster `json:"items"`
}

func init() {
	SchemeBuilder.Register(&LumaCluster{}, &LumaClusterList{})
}
