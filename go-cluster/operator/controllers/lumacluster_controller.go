package controllers

import (
	"context"

	appsv1 "k8s.io/api/apps/v1"
	corev1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/api/errors"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/log"

	lumav1alpha1 "github.com/lumadb/cluster/operator/api/v1alpha1"
)

// LumaClusterReconciler reconciles a LumaCluster object
type LumaClusterReconciler struct {
	client.Client
	Scheme *runtime.Scheme
}

// +kubebuilder:rbac:groups=luma.db,resources=lumaclusters,verbs=get;list;watch;create;update;patch;delete
// +kubebuilder:rbac:groups=luma.db,resources=lumaclusters/status,verbs=get;update;patch
// +kubebuilder:rbac:groups=apps,resources=statefulsets,verbs=get;list;watch;create;update;patch;delete
// +kubebuilder:rbac:groups=core,resources=services,verbs=get;list;watch;create;update;patch;delete

func (r *LumaClusterReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	l := log.FromContext(ctx)

	// 1. Fetch the LumaCluster instance
	lumaCluster := &lumav1alpha1.LumaCluster{}
	err := r.Get(ctx, req.NamespacedName, lumaCluster)
	if err != nil {
		if errors.IsNotFound(err) {
			return ctrl.Result{}, nil
		}
		l.Error(err, "Failed to get LumaCluster")
		return ctrl.Result{}, err
	}

	// 2. Define the desired StatefulSet
	ss := r.statefulSetForLumaCluster(lumaCluster)

	// 3. Check if StatefulSet exists
	found := &appsv1.StatefulSet{}
	err = r.Get(ctx, client.ObjectKey{Name: ss.Name, Namespace: ss.Namespace}, found)
	if err != nil && errors.IsNotFound(err) {
		l.Info("Creating a new StatefulSet", "StatefulSet.Namespace", ss.Namespace, "StatefulSet.Name", ss.Name)
		err = r.Create(ctx, ss)
		if err != nil {
			l.Error(err, "Failed to create new StatefulSet")
			return ctrl.Result{}, err
		}
		// Created successfully - return and requeue
		return ctrl.Result{Requeue: true}, nil
	} else if err != nil {
		l.Error(err, "Failed to get StatefulSet")
		return ctrl.Result{}, err
	}

	// 4. Update Status (Active Nodes)
	if found.Status.ReadyReplicas != lumaCluster.Status.ActiveNodes {
		lumaCluster.Status.ActiveNodes = found.Status.ReadyReplicas
		lumaCluster.Status.Phase = "Running"
		if err := r.Status().Update(ctx, lumaCluster); err != nil {
			l.Error(err, "Failed to update LumaCluster status")
			return ctrl.Result{}, err
		}
	}

	return ctrl.Result{}, nil
}

func (r *LumaClusterReconciler) statefulSetForLumaCluster(l *lumav1alpha1.LumaCluster) *appsv1.StatefulSet {
	labels := map[string]string{"app": "luma-db", "luma_cr": l.Name}
	replicas := l.Spec.Replicas

	ss := &appsv1.StatefulSet{
		ObjectMeta: metav1.ObjectMeta{
			Name:      l.Name + "-ss",
			Namespace: l.Namespace,
			Labels:    labels,
		},
		Spec: appsv1.StatefulSetSpec{
			Replicas: &replicas,
			Selector: &metav1.LabelSelector{
				MatchLabels: labels,
			},
			ServiceName: l.Name + "-headless",
			Template: corev1.PodTemplateSpec{
				ObjectMeta: metav1.ObjectMeta{
					Labels: labels,
				},
				Spec: corev1.PodSpec{
					Containers: []corev1.Container{{
						Name:  "luma-node",
						Image: l.Spec.Image,
						Ports: []corev1.ContainerPort{
							{ContainerPort: 8080, Name: "http"},
							{ContainerPort: 9090, Name: "grpc"},
							{ContainerPort: 10000, Name: "raft"},
						},
						Env: []corev1.EnvVar{
							{
								Name: "LUMA_NODE_ID",
								ValueFrom: &corev1.EnvVarSource{
									FieldRef: &corev1.ObjectFieldSelector{FieldPath: "metadata.name"},
								},
							},
						},
					}},
				},
			},
		},
	}
	// TODO: SetControllerReference
	return ss
}

func (r *LumaClusterReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&lumav1alpha1.LumaCluster{}).
		Owns(&appsv1.StatefulSet{}).
		Complete(r)
}
