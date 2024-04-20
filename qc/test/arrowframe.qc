// Created by Crowbar 0.68

$modelname "emb/arrowframe.mdl"

$staticprop

$bodygroup "Body"
{
	studio "arrowframe_ref.smd"
}


$surfaceprop "no_decal"

$contents "solid"

$illumposition 0.001 48.002 0

$cdmaterials "models\emb\arrowframe\"

$texturegroup "skinfamilies"
{
	{ "train_metalceiling_02" "prodcaution" "train_metalceiling_01" "white" }
}

$cbox 0 0 0 0 0 0

$bbox -400.259 -384.253 -48.25 304.248 384.252 48.25

$definebone "static_prop" "" 0 0 0 0 0 0 0 0 0 0 0 0


$sequence "idle" {
	"arrowframe_anims\idle.smd"
	fadein 0.2
	fadeout 0.2
	fps 30
}

$collisionmodel "arrowframe_physics.smd"
{
	$mass 47850.57
	$inertia 1
	$damping 0
	$rotdamping 0
	$rootbone " "
	$concave
	$maxconvexpieces 7

}
