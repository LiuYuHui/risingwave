/* DO NOT EDIT THIS FILE - it is machine generated */
#include <jni.h>
/* Header for class com_risingwave_java_binding_Binding */

#ifndef _Included_com_risingwave_java_binding_Binding
#define _Included_com_risingwave_java_binding_Binding
#ifdef __cplusplus
extern "C" {
#endif
/*
 * Class:     com_risingwave_java_binding_Binding
 * Method:    iteratorNew
 * Signature: ([B)J
 */
JNIEXPORT jlong JNICALL Java_com_risingwave_java_binding_Binding_iteratorNew
  (JNIEnv *, jclass, jbyteArray);

/*
 * Class:     com_risingwave_java_binding_Binding
 * Method:    iteratorNext
 * Signature: (J)J
 */
JNIEXPORT jlong JNICALL Java_com_risingwave_java_binding_Binding_iteratorNext
  (JNIEnv *, jclass, jlong);

/*
 * Class:     com_risingwave_java_binding_Binding
 * Method:    iteratorClose
 * Signature: (J)V
 */
JNIEXPORT void JNICALL Java_com_risingwave_java_binding_Binding_iteratorClose
  (JNIEnv *, jclass, jlong);

/*
 * Class:     com_risingwave_java_binding_Binding
 * Method:    rowGetKey
 * Signature: (J)[B
 */
JNIEXPORT jbyteArray JNICALL Java_com_risingwave_java_binding_Binding_rowGetKey
  (JNIEnv *, jclass, jlong);

/*
 * Class:     com_risingwave_java_binding_Binding
 * Method:    rowIsNull
 * Signature: (JI)Z
 */
JNIEXPORT jboolean JNICALL Java_com_risingwave_java_binding_Binding_rowIsNull
  (JNIEnv *, jclass, jlong, jint);

/*
 * Class:     com_risingwave_java_binding_Binding
 * Method:    rowGetInt16Value
 * Signature: (JI)S
 */
JNIEXPORT jshort JNICALL Java_com_risingwave_java_binding_Binding_rowGetInt16Value
  (JNIEnv *, jclass, jlong, jint);

/*
 * Class:     com_risingwave_java_binding_Binding
 * Method:    rowGetInt32Value
 * Signature: (JI)I
 */
JNIEXPORT jint JNICALL Java_com_risingwave_java_binding_Binding_rowGetInt32Value
  (JNIEnv *, jclass, jlong, jint);

/*
 * Class:     com_risingwave_java_binding_Binding
 * Method:    rowGetInt64Value
 * Signature: (JI)J
 */
JNIEXPORT jlong JNICALL Java_com_risingwave_java_binding_Binding_rowGetInt64Value
  (JNIEnv *, jclass, jlong, jint);

/*
 * Class:     com_risingwave_java_binding_Binding
 * Method:    rowGetFloatValue
 * Signature: (JI)F
 */
JNIEXPORT jfloat JNICALL Java_com_risingwave_java_binding_Binding_rowGetFloatValue
  (JNIEnv *, jclass, jlong, jint);

/*
 * Class:     com_risingwave_java_binding_Binding
 * Method:    rowGetDoubleValue
 * Signature: (JI)D
 */
JNIEXPORT jdouble JNICALL Java_com_risingwave_java_binding_Binding_rowGetDoubleValue
  (JNIEnv *, jclass, jlong, jint);

/*
 * Class:     com_risingwave_java_binding_Binding
 * Method:    rowGetBooleanValue
 * Signature: (JI)Z
 */
JNIEXPORT jboolean JNICALL Java_com_risingwave_java_binding_Binding_rowGetBooleanValue
  (JNIEnv *, jclass, jlong, jint);

/*
 * Class:     com_risingwave_java_binding_Binding
 * Method:    rowGetStringValue
 * Signature: (JI)Ljava/lang/String;
 */
JNIEXPORT jstring JNICALL Java_com_risingwave_java_binding_Binding_rowGetStringValue
  (JNIEnv *, jclass, jlong, jint);

/*
 * Class:     com_risingwave_java_binding_Binding
 * Method:    rowClose
 * Signature: (J)V
 */
JNIEXPORT void JNICALL Java_com_risingwave_java_binding_Binding_rowClose
  (JNIEnv *, jclass, jlong);

#ifdef __cplusplus
}
#endif
#endif
